use super::*;
use quick_xml::{Reader, events::Event};

fn escape(text: &str) -> String {
    quick_xml::escape::escape(text).into_owned()
}
fn paragraph(text: &str, style: Option<&str>) -> String {
    format!(
        "<w:p>{}<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        style
            .map(|s| format!("<w:pPr><w:pStyle w:val=\"{s}\"/></w:pPr>"))
            .unwrap_or_default(),
        escape(text)
    )
}
pub(super) fn write_docx(path: &Path, content: &str) -> Result<()> {
    let mut body = String::new();
    let mut in_table = false;
    for line in content.lines() {
        if line.starts_with('|') && line.ends_with('|') {
            if line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) {
                continue;
            }
            if !in_table {
                body.push_str("<w:tbl><w:tblPr><w:tblBorders><w:top w:val=\"single\" w:sz=\"4\"/><w:bottom w:val=\"single\" w:sz=\"4\"/><w:insideH w:val=\"single\" w:sz=\"4\"/><w:insideV w:val=\"single\" w:sz=\"4\"/></w:tblBorders></w:tblPr>");
                in_table = true;
            }
            body.push_str("<w:tr>");
            for cell in line.trim_matches('|').split('|') {
                body.push_str(&format!("<w:tc>{}</w:tc>", paragraph(cell.trim(), None)));
            }
            body.push_str("</w:tr>");
            continue;
        }
        if in_table {
            body.push_str("</w:tbl>");
            in_table = false;
        }
        if let Some(text) = line.strip_prefix("### ") {
            body.push_str(&paragraph(text, Some("Heading3")));
        } else if let Some(text) = line.strip_prefix("## ") {
            body.push_str(&paragraph(text, Some("Heading2")));
        } else if let Some(text) = line.strip_prefix("# ") {
            body.push_str(&paragraph(text, Some("Heading1")));
        } else if let Some(text) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            body.push_str(&format!("<w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>",escape(text)));
        } else {
            body.push_str(&paragraph(line, None));
        }
    }
    if in_table {
        body.push_str("</w:tbl>");
    }
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{body}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr></w:body></w:document>"
    );
    let styles = r#"<?xml version="1.0"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:rPr><w:sz w:val="22"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:rPr><w:b/><w:sz w:val="36"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:rPr><w:b/><w:sz w:val="30"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/><w:basedOn w:val="Normal"/><w:rPr><w:b/><w:sz w:val="26"/></w:rPr></w:style></w:styles>"#;
    let numbering = r#"<?xml version="1.0"?><w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#;
    let entries = [
        (
            "[Content_Types].xml",
            r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
        ),
        (
            "word/_rels/document.xml.rels",
            r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/></Relationships>"#,
        ),
        ("word/document.xml", &document),
        ("word/styles.xml", styles),
        ("word/numbering.xml", numbering),
    ];
    let mut zip = zip::ZipWriter::new(File::create(path)?);
    for (name, data) in entries {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .map_err(invalid)?;
        zip.write_all(data.as_bytes())?;
    }
    zip.finish().map_err(invalid)?;
    Ok(())
}
pub(super) fn read_docx(path: &Path) -> Result<String> {
    let mut zip = zip::ZipArchive::new(File::open(path)?).map_err(invalid)?;
    let entry = zip.by_name("word/document.xml").map_err(invalid)?;
    if entry.size() > 4_000_000 {
        return Err(Error::Invalid("文档正文过大，请缩小范围".into()));
    }
    let mut xml = String::new();
    entry.take(4_000_001).read_to_string(&mut xml)?;
    if xml.len() > 4_000_000 {
        return Err(Error::Invalid("文档正文过大".into()));
    }
    let mut reader = Reader::from_str(&xml);
    let mut result = String::new();
    let mut text = false;
    loop {
        match reader.read_event().map_err(invalid)? {
            Event::Start(e) => {
                if e.local_name().as_ref() == b"t" {
                    text = true;
                }
            }
            Event::Text(e) if text => result.push_str(&e.decode().map_err(invalid)?),
            Event::GeneralRef(e) if text => {
                let entity = e.decode().map_err(invalid)?;
                result.push_str(
                    &quick_xml::escape::unescape(&format!("&{entity};")).map_err(invalid)?,
                );
            }
            Event::End(e) => {
                if e.local_name().as_ref() == b"t" {
                    text = false;
                }
                if e.local_name().as_ref() == b"p" {
                    result.push('\n');
                }
            }
            Event::DocType(_) => {
                return Err(Error::Invalid("不支持带外部文档类型声明的 DOCX".into()));
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(result)
}
