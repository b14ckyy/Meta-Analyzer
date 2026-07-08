// Meta-Analyzer - AI-powered metadata tagger for photos and videos.
// Copyright (C) 2026 b14ckyy
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use anyhow::Result;
use little_exif::exif_tag::ExifTag;
use little_exif::ifd::ExifTagGroup;
use little_exif::metadata::Metadata;

/// Windows-XP tag IDs (IFD0 / GENERIC group), stored as UTF-16LE byte arrays.
const TAG_XP_KEYWORDS: u16 = 0x9C9E;
const TAG_XP_SUBJECT: u16 = 0x9C9F;

/// JPEG markers
const APP1_MARKER: u8 = 0xE1;   // EXIF and XMP both live here
const APP13_MARKER: u8 = 0xED;  // Photoshop IRB (hosts IPTC)

/// Segment-identifying prefixes.
const XMP_NS_SIGNATURE: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const PHOTOSHOP_SIGNATURE: &[u8] = b"Photoshop 3.0\0";

/// Photoshop IRB resource ID for IPTC-NAA records.
const IPTC_RESOURCE_ID: u16 = 0x0404;

/// IPTC dataset constants we care about.
const IPTC_ENVELOPE_RECORD: u8 = 0x01;
const IPTC_APPLICATION_RECORD: u8 = 0x02;
const IPTC_CODED_CHARACTER_SET: u8 = 0x5A; // 1:90
const IPTC_KEYWORDS: u8 = 0x19;            // 2:25

/// Read any existing tags from an image file.
/// For JPEG: tries IPTC Keywords (APP13/2:25) first, then EXIF ImageDescription.
/// For other formats: tries EXIF ImageDescription only.
pub fn read_tags(image_path: &str) -> Vec<String> {
    let ext = std::path::Path::new(image_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "jpg" || ext == "jpeg" {
        let iptc = read_iptc_keywords_jpeg(image_path);
        if !iptc.is_empty() {
            return iptc;
        }
    }

    read_exif_description(image_path)
}

fn read_exif_description(image_path: &str) -> Vec<String> {
    let path = std::path::Path::new(image_path);
    let metadata = match Metadata::new_from_path(path) {
        Ok(m) => m,
        Err(_) => return vec![],
    };
    for tag in &metadata {
        if let ExifTag::ImageDescription(desc) = tag {
            let tags: Vec<String> = desc
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            if !tags.is_empty() {
                return tags;
            }
        }
    }
    vec![]
}

fn read_iptc_keywords_jpeg(image_path: &str) -> Vec<String> {
    let data = match std::fs::read(image_path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let jpeg = match img_parts::jpeg::Jpeg::from_bytes(img_parts::Bytes::from(data)) {
        Ok(j) => j,
        Err(_) => return vec![],
    };
    let app13 = match jpeg
        .segments()
        .iter()
        .find(|s| s.marker() == APP13_MARKER && s.contents().starts_with(PHOTOSHOP_SIGNATURE))
    {
        Some(s) => s,
        None => return vec![],
    };
    let body = match app13.contents().strip_prefix(PHOTOSHOP_SIGNATURE) {
        Some(b) => b,
        None => return vec![],
    };

    let mut pos = 0;
    while pos + 12 <= body.len() {
        if &body[pos..pos + 4] != b"8BIM" {
            break;
        }
        let resource_id = u16::from_be_bytes([body[pos + 4], body[pos + 5]]);
        let name_len = body[pos + 6] as usize;
        let name_total = 1 + name_len;
        let name_padded = if name_total % 2 == 0 { name_total } else { name_total + 1 };
        let name_end = pos + 6 + name_padded;
        if name_end + 4 > body.len() { break; }
        let size = u32::from_be_bytes([
            body[name_end], body[name_end + 1], body[name_end + 2], body[name_end + 3],
        ]) as usize;
        let data_start = name_end + 4;
        let data_end = data_start + size;
        if data_end > body.len() { break; }
        let block_end = data_end + (size % 2);

        if resource_id == IPTC_RESOURCE_ID {
            return extract_iptc_keywords(&body[data_start..data_end]);
        }
        pos = block_end;
    }
    vec![]
}

fn extract_iptc_keywords(iptc_data: &[u8]) -> Vec<String> {
    let mut keywords = Vec::new();
    let mut pos = 0;
    while pos + 5 <= iptc_data.len() {
        if iptc_data[pos] != 0x1C { break; }
        let record = iptc_data[pos + 1];
        let dataset = iptc_data[pos + 2];
        let first_len_byte = iptc_data[pos + 3];

        let (value_start, value_len) = if first_len_byte & 0x80 != 0 {
            let hdr_len_bytes =
                u16::from_be_bytes([first_len_byte & 0x7F, iptc_data[pos + 4]]) as usize;
            if pos + 5 + hdr_len_bytes > iptc_data.len() { break; }
            let vl = match hdr_len_bytes {
                1 => iptc_data[pos + 5] as usize,
                2 => u16::from_be_bytes([iptc_data[pos + 5], iptc_data[pos + 6]]) as usize,
                _ => break,
            };
            (pos + 5 + hdr_len_bytes, vl)
        } else {
            let vl = u16::from_be_bytes([first_len_byte, iptc_data[pos + 4]]) as usize;
            (pos + 5, vl)
        };

        let end = value_start + value_len;
        if end > iptc_data.len() { break; }

        if record == IPTC_APPLICATION_RECORD && dataset == IPTC_KEYWORDS {
            if let Ok(kw) = std::str::from_utf8(&iptc_data[value_start..end]) {
                let kw = kw.trim().to_string();
                if !kw.is_empty() {
                    keywords.push(kw);
                }
            }
        }
        pos = end;
    }
    keywords
}

/// Write AI-generated tags into the image file.
///
/// Writes multiple formats for cross-tool compatibility:
/// 1. **EXIF** via little_exif — `ImageDescription`, `XPKeywords`, `XPSubject`, `Software`
/// 2. **XMP `dc:subject`** (JPEG) — Adobe/Lightroom standard for keywords
/// 3. **IPTC Keywords** (JPEG, APP13 / 2:25) — **what Plex uses for its "Schlagworte" field**
///
/// The APP13 segment is preserved: existing Photoshop resources (clipping paths etc.) and
/// existing IPTC records (Copyright, Byline, …) are kept intact — only the Keywords records
/// are replaced with ours.
pub fn write_metadata(image_path: &str, tags: &[String]) -> Result<()> {
    write_exif_tags(image_path, tags)?;

    let ext = std::path::Path::new(image_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "jpg" || ext == "jpeg" {
        write_xmp_and_iptc_jpeg(image_path, tags)?;
    }
    Ok(())
}

fn write_exif_tags(image_path: &str, tags: &[String]) -> Result<()> {
    let path_obj = std::path::Path::new(image_path);
    let mut metadata = match Metadata::new_from_path(path_obj) {
        Ok(m) => m,
        Err(_) => Metadata::new(),
    };

    let csv_tags = tags.join(", ");
    let semi_tags = tags.join(";");

    metadata.set_tag(ExifTag::ImageDescription(csv_tags.clone()));
    metadata.set_tag(ExifTag::Software("Meta-Analyzer".to_string()));
    metadata.set_tag(ExifTag::UnknownINT8U(
        encode_utf16le_nullterm(&semi_tags),
        TAG_XP_KEYWORDS,
        ExifTagGroup::GENERIC,
    ));
    metadata.set_tag(ExifTag::UnknownINT8U(
        encode_utf16le_nullterm(&csv_tags),
        TAG_XP_SUBJECT,
        ExifTagGroup::GENERIC,
    ));

    metadata.write_to_file(path_obj)?;
    Ok(())
}

/// Rebuild XMP (APP1) and Photoshop IRB (APP13) segments, preserving anything
/// unrelated to our keywords.
fn write_xmp_and_iptc_jpeg(image_path: &str, tags: &[String]) -> Result<()> {
    let data = std::fs::read(image_path)?;
    let mut jpeg = img_parts::jpeg::Jpeg::from_bytes(img_parts::Bytes::from(data))?;

    // Build the new APP13 content BEFORE mutating segments, so we can read the
    // existing Photoshop IRB to preserve its resources.
    let new_app13_content: Vec<u8> = {
        let existing = jpeg
            .segments()
            .iter()
            .find(|s| s.marker() == APP13_MARKER && s.contents().starts_with(PHOTOSHOP_SIGNATURE))
            .map(|s| s.contents().to_vec());
        build_photoshop_irb_with_keywords(existing.as_deref(), tags)
    };

    // Remove old XMP APP1 and old Photoshop APP13 — we'll re-insert fresh ones.
    jpeg.segments_mut().retain(|seg| {
        let marker = seg.marker();
        if marker == APP1_MARKER && seg.contents().starts_with(XMP_NS_SIGNATURE) {
            return false;
        }
        if marker == APP13_MARKER && seg.contents().starts_with(PHOTOSHOP_SIGNATURE) {
            return false;
        }
        true
    });

    // Build XMP APP1
    let xmp_xml = build_xmp_packet(tags);
    let mut xmp_content = Vec::with_capacity(XMP_NS_SIGNATURE.len() + xmp_xml.len());
    xmp_content.extend_from_slice(XMP_NS_SIGNATURE);
    xmp_content.extend_from_slice(xmp_xml.as_bytes());
    let xmp_segment = img_parts::jpeg::JpegSegment::new_with_contents(
        APP1_MARKER,
        img_parts::Bytes::from(xmp_content),
    );

    // Build APP13
    let app13_segment = img_parts::jpeg::JpegSegment::new_with_contents(
        APP13_MARKER,
        img_parts::Bytes::from(new_app13_content),
    );

    // Insert both after the last APP1 (keeps EXIF APP1 first).
    let segments = jpeg.segments_mut();
    let mut insert_at = segments
        .iter()
        .rposition(|s| s.marker() == APP1_MARKER)
        .map(|p| p + 1)
        .unwrap_or(1);
    segments.insert(insert_at, xmp_segment);
    insert_at += 1;
    segments.insert(insert_at, app13_segment);

    let mut output = std::io::Cursor::new(Vec::new());
    jpeg.encoder().write_to(&mut output)?;
    std::fs::write(image_path, output.into_inner())?;
    Ok(())
}

/// Build the full Photoshop IRB (APP13 content) with our Keywords merged in.
/// Preserves every existing 8BIM resource; rebuilds the IPTC-NAA resource only.
fn build_photoshop_irb_with_keywords(existing: Option<&[u8]>, tags: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(PHOTOSHOP_SIGNATURE);

    // Capture the existing IPTC-NAA data (if any) so we can preserve records other than Keywords.
    let mut existing_iptc: Vec<u8> = Vec::new();

    if let Some(data) = existing {
        if let Some(body) = data.strip_prefix(PHOTOSHOP_SIGNATURE) {
            let mut pos = 0;
            while pos + 12 <= body.len() {
                if &body[pos..pos + 4] != b"8BIM" {
                    break;
                }
                let resource_id = u16::from_be_bytes([body[pos + 4], body[pos + 5]]);
                let name_len = body[pos + 6] as usize;
                let name_total = 1 + name_len;
                let name_padded = if name_total % 2 == 0 { name_total } else { name_total + 1 };
                let name_end = pos + 6 + name_padded;
                if name_end + 4 > body.len() {
                    break;
                }
                let size = u32::from_be_bytes([
                    body[name_end],
                    body[name_end + 1],
                    body[name_end + 2],
                    body[name_end + 3],
                ]) as usize;
                let data_start = name_end + 4;
                let data_end = data_start + size;
                if data_end > body.len() {
                    break;
                }
                let block_end = data_end + (size % 2); // pad to even

                if resource_id == IPTC_RESOURCE_ID {
                    // Capture IPTC bytes, rebuild this resource at the end.
                    existing_iptc.extend_from_slice(&body[data_start..data_end]);
                } else {
                    // Verbatim-copy any non-IPTC 8BIM (paths, slices, ICC ref, etc.).
                    out.extend_from_slice(&body[pos..block_end.min(body.len())]);
                }

                pos = block_end;
            }
        }
    }

    // Build the (updated) IPTC-NAA dataset stream
    let iptc_data = rebuild_iptc_records(&existing_iptc, tags);

    // Emit the IPTC 8BIM resource
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(&IPTC_RESOURCE_ID.to_be_bytes());
    out.extend_from_slice(&[0x00, 0x00]); // empty Pascal-string name, already even
    out.extend_from_slice(&(iptc_data.len() as u32).to_be_bytes());
    out.extend_from_slice(&iptc_data);
    if iptc_data.len() % 2 != 0 {
        out.push(0x00);
    }

    out
}

/// Walk existing IPTC dataset stream, drop CodedCharacterSet (1:90) and Keywords (2:25),
/// keep everything else. Prepend fresh UTF-8 declaration, append our new Keywords records.
fn rebuild_iptc_records(existing: &[u8], tags: &[String]) -> Vec<u8> {
    let mut out = Vec::new();

    // 1:90 CodedCharacterSet = ESC % G → UTF-8. Tells readers the strings are UTF-8.
    out.extend_from_slice(&[
        0x1C, IPTC_ENVELOPE_RECORD, IPTC_CODED_CHARACTER_SET,
        0x00, 0x03,
        0x1B, 0x25, 0x47,
    ]);

    // Walk existing records, skipping the two we're re-writing
    let mut pos = 0;
    while pos + 5 <= existing.len() {
        if existing[pos] != 0x1C {
            break;
        }
        let record = existing[pos + 1];
        let dataset = existing[pos + 2];
        let first_len_byte = existing[pos + 3];

        let (value_start, value_len) = if first_len_byte & 0x80 != 0 {
            // Extended length: header has `hdr_len_bytes` bytes encoding the value length.
            let hdr_len_bytes = u16::from_be_bytes([first_len_byte & 0x7F, existing[pos + 4]]) as usize;
            if pos + 5 + hdr_len_bytes > existing.len() { break; }
            let vl = match hdr_len_bytes {
                1 => existing[pos + 5] as usize,
                2 => u16::from_be_bytes([existing[pos + 5], existing[pos + 6]]) as usize,
                4 => u32::from_be_bytes([
                    existing[pos + 5], existing[pos + 6], existing[pos + 7], existing[pos + 8],
                ]) as usize,
                _ => break,
            };
            (pos + 5 + hdr_len_bytes, vl)
        } else {
            let vl = u16::from_be_bytes([first_len_byte, existing[pos + 4]]) as usize;
            (pos + 5, vl)
        };
        let end = value_start + value_len;
        if end > existing.len() {
            break;
        }

        let skip = (record == IPTC_ENVELOPE_RECORD && dataset == IPTC_CODED_CHARACTER_SET)
            || (record == IPTC_APPLICATION_RECORD && dataset == IPTC_KEYWORDS);

        if !skip {
            out.extend_from_slice(&existing[pos..end]);
        }
        pos = end;
    }

    // Append our Keywords records (2:25). One IPTC record per tag.
    for tag in tags {
        let bytes = tag.as_bytes();
        let len = bytes.len().min(u16::MAX as usize) as u16;
        out.push(0x1C);
        out.push(IPTC_APPLICATION_RECORD);
        out.push(IPTC_KEYWORDS);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&bytes[..len as usize]);
    }

    out
}

fn build_xmp_packet(tags: &[String]) -> String {
    let items = tags
        .iter()
        .map(|t| format!("     <rdf:li>{}</rdf:li>", xml_escape(t)))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Meta-Analyzer\">\n\
 <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
  <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
   <dc:subject>\n\
    <rdf:Bag>\n\
{items}\n\
    </rdf:Bag>\n\
   </dc:subject>\n\
  </rdf:Description>\n\
 </rdf:RDF>\n\
</x:xmpmeta>\n\
<?xpacket end=\"w\"?>"
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn encode_utf16le_nullterm(s: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(s.len() * 2 + 2);
    for unit in s.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes.extend_from_slice(&[0, 0]);
    bytes
}
