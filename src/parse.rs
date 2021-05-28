use super::data::{ContentOptions, Picture, Presentation, Slide, Title};
use super::Error;
use std::io::Read;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until};
use nom::multi::{many0, many1};
use nom::sequence::tuple;
use nom::IResult;

type NomErr<'a> = nom::Err<nom::error::Error<&'a str>>;

pub(crate) fn parse_markdown<R: Read>(mut reader: R) -> Result<Presentation, Error> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let text = String::from_utf8(buffer)?;

    let (_, presentation) = inner_parse(&text).map_err(|_| Error::NomError)?;
    Ok(presentation)
}

fn inner_parse<'a>(i: &'a str) -> IResult<&'a str, Presentation> {
    let (rest, (title, author)) = parse_start_header(i)?;

    let (rest, slides) = many0(parse_slide)(rest)?;

    Ok((
        rest,
        Presentation {
            title,
            author,
            slides,
        },
    ))
}

fn parse_start_header<'a>(i: &'a str) -> IResult<&'a str, (Title, String)> {
    let (rest, _) = take_till(|c| c == '#')(i)?;
    let (rest, _) = tag("# ")(rest)?;
    let (rest, title_name) = take_till(|c| c == '\n')(rest)?;
    let (author_start, _whitespace) = tag("\nAUTHOR=")(rest)?;
    let (rest, author_name) = take_till(|c| c == '\n')(author_start)?;

    let title_spans = parse_string(title_name)?;

    Ok((
        rest,
        (Title { title: title_spans }, author_name.to_string()),
    ))
}

fn parse_slide<'a>(i: &'a str) -> IResult<&'a str, Slide> {
    let (title_start, _) = tuple((take_until("##"), tag("## ")))(i)?;

    let (rest, slide_title) = take_till(|c| c == '\n')(title_start)?;
    let (rest, blocks) = parse_block(rest)?;

    let (pictures, non_pictures): (Vec<_>, Vec<_>) =
        blocks.into_iter().partition(|b| b.is_picture());

    let contents = match (pictures.into_iter().next(), non_pictures.len()) {
        (Some(picture), 0) => ContentOptions::OnlyPicture(picture.unwrap_picture()),
        (Some(picture), _) => {
            ContentOptions::TextAndPicture(non_pictures, picture.unwrap_picture())
        }
        (None, _) => ContentOptions::OnlyText(non_pictures),
    };

    let slide = Slide {
        title: Title {
            title: parse_string(slide_title)?,
        },
        contents,
    };

    Ok((rest, slide))
}

// TODO: stop conditions for pictures
fn parse_block<'a>(i: &'a str) -> IResult<&'a str, Vec<Block>> {
    let alt_parser = |x: &'a str| -> IResult<&'a str, &'a str> {
        alt((take_until("\n\n"), take_until("\n")))(x)
    };
    let whitespace = take_till(|c| c != ' ' && c != '\n' && c != '\t');

    let block_stop = tuple((whitespace, alt_parser));

    let whitespace = take_till(|c| c != ' ' && c != '\n' && c != '\t');

    let end_of_slide = tuple((whitespace, alt((nom::combinator::eof, is_start_header))));

    let (rest, (block_whitespace_iter, _)) = nom::multi::many_till(block_stop, end_of_slide)(i)?;

    let blocks: Vec<Block> = block_whitespace_iter
        .into_iter()
        .map(|x| x.1)
        .map(|block_text| {
            alt((
                parse_block_as_picture,
                parse_block_as_bullets,
                parse_block_as_paragraph,
            ))(block_text)
        })
        .map(|x| x.unwrap().1)
        .collect();

    Ok((rest, blocks))
}

fn is_start_header(i: &str) -> IResult<&str, &str> {
    match tag("##")(i) {
        Ok(_) => Ok((i, "")),
        Err(e) => Err(e),
    }
}

fn parse_block_as_paragraph(i: &str) -> IResult<&str, Block> {
    let spans = parse_string(i)?;
    Ok(("", Block::Paragraph(spans)))
}

fn parse_block_as_picture(i: &str) -> IResult<&str, Block> {
    let (caption_start, _) = tag("![")(i)?;
    let (location_paren_start, caption) = take_till(|c| c == ']')(caption_start)?;
    let (location_start, _) = tag("](")(location_paren_start)?;
    let (rest, location) = take_till(|c| c == ')')(location_start)?;

    let caption = caption.to_string();
    let location = location.to_string();

    let picture = if location.starts_with("http") {
        Picture::Link {
            caption,
            link: location,
        }
    } else {
        Picture::Path {
            caption,
            path: location,
        }
    };

    Ok((rest, Block::Picture(picture)))
}

fn parse_block_as_bullets(i: &str) -> IResult<&str, Block> {
    let take_whitespace = take_till(|c| c != '\n');

    let (_, bullets) = many1(tuple((take_whitespace, parse_bullet_item)))(i)?;

    let mut bullets: Vec<(usize, BulletItem)> = bullets.into_iter().map(|x| x.1).collect();

    let mut organized = Vec::new();

    collect_bullet_items(&mut bullets, &mut organized, 0);

    Ok(("", Block::BulletedList(organized)))
}

fn parse_bullet_item(i: &str) -> IResult<&str, (usize, BulletItem)> {
    let (rest, taken) = take_till(|c| c != ' ' && c != '\t')(i)?;

    let current_indentation = taken.chars().fold(0, |acc, x| {
        if x == ' ' {
            acc + 1
        } else if x == '\t' {
            acc + 4
        } else {
            acc
        }
    }) / 4;

    let (rest, (_, bullet_text)) =
        tuple((tag("* "), alt((take_until("\n"), nom::combinator::rest))))(rest)?;

    Ok((
        rest,
        (
            current_indentation,
            BulletItem::Single(parse_string(bullet_text)?),
        ),
    ))
}

fn collect_bullet_items(
    flat: &mut Vec<(usize, BulletItem)>,
    nested: &mut Vec<BulletItem>,
    current_indentation: usize,
) {
    loop {
        if flat.len() > 0 {
            let (indentation, _bullet_data) = &flat[0];
            // copy the data for borrowing rules
            let indentation = *indentation;

            if indentation > current_indentation {
                let mut new_buffer = Vec::new();
                collect_bullet_items(flat, &mut new_buffer, indentation);
                nested.push(BulletItem::Nested(new_buffer));
            } else if indentation < current_indentation {
                // return back to the previous level of indentation
                return;
            } else {
                // we have the same level of indentation
                let item = flat.remove(0);
                nested.push(item.1)
            }
        } else {
            break;
        }
    }
}

fn parse_string<'a>(i: &'a str) -> Result<Vec<Span>, NomErr> {
    let span_options = |x: &'a str| {
        let alts = alt((
            parse_strikethrough,
            parse_bold,
            parse_italics,
            parse_equation,
            parse_regular_text,
        ))(x);
        alts
    };

    let out = nom::multi::many1(span_options)(i)?;

    Ok(out.1)
}

// TODO: does not handle escaped sequences
fn parse_bold<'a>(i: &'a str) -> IResult<&'a str, Span> {
    let (rest, (_, bolded_text, _)) = tuple((
        //
        tag("**"),
        take_until("**"),
        tag("**"),
    ))(i)?;

    Ok((rest, Span::Bold(bolded_text.to_string())))
}

// TODO: does not handle escaped sequences
fn parse_strikethrough<'a>(i: &'a str) -> IResult<&'a str, Span> {
    let (rest, (_, strikethrough, _)) = tuple((
        //
        tag("~~"),
        take_until("~~"),
        tag("~~"),
    ))(i)?;

    Ok((rest, Span::Strikethrough(strikethrough.to_string())))
}

// TODO: does not handle escaped sequences
fn parse_italics<'a>(i: &'a str) -> IResult<&'a str, Span> {
    let (rest, (_, italics, _)) = tuple((
        //
        tag("*"),
        take_until("*"),
        tag("*"),
    ))(i)?;

    Ok((rest, Span::Italics(italics.to_string())))
}

// TODO: does not handle escaped sequences
fn parse_equation<'a>(i: &'a str) -> IResult<&'a str, Span> {
    let (rest, (_, italics, _)) = tuple((
        //
        tag("$$"),
        take_until("$$"),
        tag("$$"),
    ))(i)?;

    Ok((rest, Span::Equation(italics.to_string())))
}

fn parse_regular_text<'a>(i: &'a str) -> IResult<&'a str, Span> {
    if i.len() == 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            "",
            nom::error::ErrorKind::Eof,
        )));
    }

    let (rest, text) =
        take_until_parser_success(i, alt((parse_italics, parse_bold, parse_strikethrough)))?;

    Ok((rest, Span::Text(text.to_string())))
}

fn take_until_parser_success<'a, T, ParserOutput>(
    i: &'a str,
    mut parser: T,
) -> IResult<&'a str, &'a str>
where
    T: nom::Parser<&'a str, ParserOutput, nom::error::Error<&'a str>>,
    T: FnMut(&'a str) -> IResult<&'a str, ParserOutput>,
{
    let mut current_slice = i;
    let mut idx = 0;

    loop {
        if let Ok(_) = parser(current_slice) {
            break;
        }

        idx += 1;

        if let Some(slice) = i.get(idx..) {
            current_slice = slice
        } else {
            idx -= 1;
            break;
        }
    }

    Ok((current_slice, i.get(0..idx).unwrap()))
}

#[derive(Debug, PartialEq)]
pub(crate) enum Block {
    Paragraph(Vec<Span>),
    BulletedList(Vec<BulletItem>),
    Picture(Picture),
}
impl Block {
    fn is_picture(&self) -> bool {
        if let Block::Picture(_) = &self {
            true
        } else {
            false
        }
    }
    fn unwrap_picture(self) -> Picture {
        if let Block::Picture(picture) = self {
            picture
        } else {
            panic!("tried to unwrap a paragraph / bullet list block into a picture ")
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum BulletItem {
    // a single bullet point that is at the same indentation level
    Single(Vec<Span>),
    // a set of bullet points at one more indentation level than the current level
    Nested(Vec<BulletItem>),
}

#[derive(Debug, PartialEq)]
pub(crate) enum Span {
    Bold(String),          //
    Strikethrough(String), //
    Italics(String),       //
    Text(String),          //
    Equation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_1() {
        let text = "**some bold thing here** other stuff";
        let out = parse_bold(text);

        let out = out.unwrap();

        assert_eq!(out.1, Span::Bold("some bold thing here".into()));
        assert_eq!(out.0, " other stuff");
    }

    #[test]
    #[ignore]
    fn bold_escapted() {
        let text = "**some bold thing here \\* and continues**";
        let out = parse_bold(text);
        dbg!(&out);
        out.unwrap();
    }

    #[test]
    fn strikethrough_1() {
        let text = "~~some strikethrough thing here~~ other stuff";
        let out = parse_strikethrough(text);

        let out = out.unwrap();

        assert_eq!(
            out.1,
            Span::Strikethrough("some strikethrough thing here".into())
        );

        assert_eq!(out.0, " other stuff");
    }

    #[test]
    fn italics_1() {
        let text = "*some italic thing here* other stuff";
        let out = parse_italics(text);

        let out = out.unwrap();

        assert_eq!(out.1, Span::Italics("some italic thing here".into()));

        assert_eq!(out.0, " other stuff");
    }

    #[test]
    fn text_1() {
        let text = "start text **bold text**";
        let out = parse_regular_text(text);
        dbg!(&out);
        let out = out.unwrap();
        assert_eq!(out.1, Span::Text("start text ".to_string()));
        assert_eq!(out.0, "**bold text**");
    }

    #[test]
    fn text_2_only_text() {
        let text = "only text";
        let out = parse_regular_text(text);
        dbg!(&out);
        let out = out.unwrap();
        assert_eq!(out.1, Span::Text("only text".to_string()));
        assert_eq!(out.0, "");
    }

    #[test]
    fn block_1() {
        let text = "\n\nblock text here\n\nanother block of something here \n## a new header here";
        let out = parse_block(text);
        dbg!(&out);
        let out = out.unwrap();
    }

    #[test]
    fn parse_string_1() {
        let text = "something **bold** *italics* ~~strike~~";
        let out = parse_string(text);
        dbg!(&out);
        let out = out.unwrap();

        let expected = vec![
            Span::Text("something ".into()),
            Span::Bold("bold".into()),
            Span::Text(" ".into()),
            Span::Italics("italics".into()),
            Span::Text(" ".into()),
            Span::Strikethrough("strike".into()),
        ];

        assert_eq!(out, expected);
    }

    #[test]
    fn parse_single_bullet_1() {
        let text = "* bullet text";
        let out = parse_bullet_item(text);
        dbg!(&out);
        let out = out.unwrap();
    }

    #[test]
    fn parse_unnested_bullet_block() {
        let text = "* bullet text\n* bullet text\n* **bolded bullet text**";
        let out = parse_block_as_bullets(text);
        dbg!(&out);
        let out = out.unwrap();
        let expected = Block::BulletedList(vec![
            BulletItem::Single(vec![Span::Text("bullet text".into())]),
            BulletItem::Single(vec![Span::Text("bullet text".into())]),
            BulletItem::Single(vec![Span::Bold("bolded bullet text".into())]),
        ]);
        assert_eq!(out.1, expected);
    }

    #[test]
    fn parse_nested_bullet_block() {
        let text = r#"
* item
* item
    * nested item
    * nested item
* regular item
        "#;
        println!("{}", text);

        let out = parse_block_as_bullets(text);
        dbg!(&out);
        let out = out.unwrap();
        let expected = Block::BulletedList(vec![
            BulletItem::Single(vec![Span::Text("item".into())]),
            BulletItem::Single(vec![Span::Text("item".into())]),
            BulletItem::Nested(vec![
                BulletItem::Single(vec![Span::Text("nested item".into())]),
                BulletItem::Single(vec![Span::Text("nested item".into())]),
            ]),
            BulletItem::Single(vec![Span::Text("regular item".into())]),
        ]);

        assert_eq!(out.1, expected)
    }

    #[test]
    fn get_header() {
        let text = r#"
# Presentation Title
AUTHOR=Author Name

        "#;

        let out = parse_start_header(text);
        dbg!(&out);
        let (_, (title, author)) = out.unwrap();
        let expected_title = Title {
            title: vec![Span::Text("Presentation Title".to_string())],
        };
        assert_eq!(title, expected_title);
        assert_eq!(author, "Author Name");
    }

    #[test]
    fn parse_slide_1() {
        let text = r#"
## Slide Title

some inner text

some more text

"#;

        let out = parse_slide(text);
        dbg!(&out);
        let out = out.unwrap();

        let expected = ContentOptions::OnlyText(vec![
            Block::Paragraph(vec![Span::Text("some inner text".to_string())]),
            Block::Paragraph(vec![Span::Text("some more text".to_string())]),
        ]);

        assert_eq!(out.1.contents, expected);
    }

    #[test]
    fn parse_picture_1() {
        let text = "![caption](path)";
        let out = parse_block_as_picture(text);
        dbg!(&out);
        let out = out.unwrap().1;
        let expected = Picture::Path {
            path: "path".to_string(),
            caption: "caption".to_string(),
        };
        assert_eq!(out.unwrap_picture(), expected);
    }
}
