use super::data::{Code, ContentOptions, Picture, Presentation, Slide, Title};
use super::Error;
use std::cmp::Ordering;
use std::io::Read;
use std::path::PathBuf;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until};
use nom::multi::{many0, many1};
use nom::sequence::tuple;
use nom::IResult;

type NomErr<'a> = nom::Err<nom::error::Error<&'a str>>;

#[derive(Debug)]
pub(crate) struct Document {
    pub(crate) first: ParsedTitle,
    pub(crate) slides: Vec<ParsedSlide>,
}

#[derive(Debug)]
pub(crate) struct ParsedTitle {
    pub(crate) title: Vec<Span>,
    pub(crate) author: String,
}

#[derive(Debug)]
pub(crate) struct ParsedSlide {
    pub(crate) title: Vec<Span>,
    pub(crate) contents: Vec<Block>,
}

pub(crate) fn parse_markdown<R: Read>(mut reader: R) -> Result<Document, Error> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let text = String::from_utf8(buffer)?;

    let (_, presentation) = inner_parse(&text).map_err(|_| Error::Nom)?;
    dbg!(&presentation);
    Ok(presentation)
}

fn inner_parse(i: &'_ str) -> IResult<&'_ str, Document> {
    let (rest, first) = parse_start_header(i)?;

    let (rest, slides) = many0(parse_slide)(rest)?;

    Ok((rest, Document { first, slides }))
}

fn parse_start_header(i: &'_ str) -> IResult<&'_ str, ParsedTitle> {
    let (rest, _) = take_till(|c| c == '#')(i)?;
    let (rest, _) = tag("# ")(rest)?;
    let (rest, title_name) = take_till(|c| c == '\n')(rest)?;
    let (author_start, _whitespace) = tag("\nAUTHOR=")(rest)?;
    let (rest, author_name) = take_till(|c| c == '\n')(author_start)?;

    let title_spans = parse_string(title_name)?;

    Ok((
        rest,
        ParsedTitle {
            title: title_spans,
            author: author_name.to_string(),
        },
    ))
}

fn parse_slide(i: &'_ str) -> IResult<&'_ str, ParsedSlide> {
    let (title_start, _) = tuple((take_until("##"), tag("## ")))(i)?;

    let (rest, slide_title) = take_till(|c| c == '\n')(title_start)?;
    let (rest, blocks) = parse_block(rest)?;

    let slide = ParsedSlide {
        title: parse_string(slide_title)?,
        contents: blocks,
    };

    Ok((rest, slide))
}

// TODO: stop conditions for pictures
fn parse_block<'a>(i: &'a str) -> IResult<&'a str, Vec<Block>> {
    let whitespace = take_till(|c| c != ' ' && c != '\n' && c != '\t');

    let end_of_slide = tuple((whitespace, alt((nom::combinator::eof, is_start_header))));

    let whitespace = take_till(|c| c != ' ' && c != '\n' && c != '\t');

    let (rest, (blocks, _)): (_, (Vec<(&str, Block)>, _)) = nom::multi::many_till(
        tuple((
            whitespace,
            alt((
                parse_as_directive,
                parse_block_as_picture,
                parse_block_as_bullets,
                parse_as_code,
                parse_block_as_paragraph,
            )),
        )),
        end_of_slide,
    )(i)?;

    let blocks = blocks.into_iter().map(|x| x.1).collect();

    Ok((rest, blocks))
}

fn is_start_header(i: &str) -> IResult<&str, &str> {
    match tag("##")(i) {
        Ok(_) => Ok((i, "")),
        Err(e) => Err(e),
    }
}

fn parse_block_as_paragraph(i: &str) -> IResult<&str, Block> {
    let (rest, before_block_end) = take_until_parser_success(i, tag("\n\n"))?;
    let spans = parse_string(before_block_end)?;
    Ok((rest, Block::Paragraph(spans)))
}

fn parse_block_as_picture(i: &str) -> IResult<&str, Block> {
    // TODO: parse a directive after the picture has been taken
    let (caption_start, _) = tag("![")(i)?;
    let (location_paren_start, caption) = take_till(|c| c == ']')(caption_start)?;
    let (location_start, _) = tag("](")(location_paren_start)?;
    let (rest, location) = take_till(|c| c == ')')(location_start)?;
    let (mut rest, _) = tag(")")(rest)?;

    println!("searching for picture directives");

    let directive = if let Ok((new_rest, directive)) = many0(picture_directive)(rest) {
        if directive.len() == 0 {
            None
        } else {
            rest = new_rest;
            Some(directive)
        }
    } else {
        None
    };

    let caption = if caption.len() == 0 {
        None
    } else {
        Some(caption.to_string())
    };

    let location = String::from(location);

    let picture = ParsePicture {
        path: location,
        directive,
        caption,
    };

    Ok((rest, Block::Picture(picture)))
}

fn picture_directive(i: &str) -> IResult<&str, PictureDirective> {
    let whitespace = take_till(|c| c != ' ' && c != '\n' && c != '\t');
    let (after_whitespace, _) = whitespace(i)?;

    let vertical = |i| -> IResult<&str, PictureDirective> {
        let (rest, _) = tag("%VERTICAL")(i)?;
        Ok((rest, PictureDirective::Vertical))
    };

    let width = |i| -> IResult<&str, PictureDirective> {
        let (rest, _) = tag("%WIDTH=")(i)?;
        let (rest, width_query) = take_until("\n")(rest)?;

        Ok((rest, PictureDirective::Width(width_query.to_string())))
    };

    let height = |i| -> IResult<&str, PictureDirective> {
        let (rest, _) = tag("%HEIGHT=")(i)?;
        let (rest, width_query) = take_until("\n")(rest)?;

        Ok((rest, PictureDirective::Width(width_query.to_string())))
    };

    let (rest, directive) = alt((vertical, width, height))(after_whitespace)?;

    Ok((rest, directive))
}

fn parse_block_as_bullets(i: &str) -> IResult<&str, Block> {
    let take_whitespace = take_till(|c| c != '\n');

    let (rest, bullets) = many1(tuple((take_whitespace, parse_bullet_item)))(i)?;

    let mut bullets: Vec<(usize, BulletItem)> = bullets.into_iter().map(|x| x.1).collect();

    let mut organized = Vec::new();

    collect_bullet_items(&mut bullets, &mut organized, 0);

    Ok((rest, Block::BulletedList(organized)))
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

fn parse_as_code(i: &str) -> IResult<&str, Block> {
    let (rest, _whitespace) = take_till(|c| c != '\n')(i)?;
    let (code_internal, header) = code_block_header(rest)?;

    let (rest, code) = take_until_parser_success(code_internal, tag("```"))?;
    let (rest, _code_end) = tag("```")(rest)?;

    Ok((
        rest,
        Block::Code(Code::new(code.to_string(), header.language)),
    ))
}

fn parse_as_directive(i: &str) -> IResult<&str, Block> {
    let (rest, _whitespace) = take_till(|c| c != '\n')(i)?;

    let (rest, _newslide_header) = tag("%NEWSLIDE")(rest)?;

    Ok((rest, Block::Directive(Directive::NewSlide)))
}

#[derive(Debug)]
struct CodeHeader {
    language: String,
}

fn code_block_header(i: &str) -> IResult<&str, CodeHeader> {
    let (after_ticks, _ticks) = tag("```")(i)?;
    let (rest, language_name) = take_till(|c| c == '\n')(after_ticks)?;
    let (code_start, _newline) = tag("\n")(rest)?;

    Ok((
        code_start,
        CodeHeader {
            language: language_name.to_string(),
        },
    ))
}

fn collect_bullet_items(
    flat: &mut Vec<(usize, BulletItem)>,
    nested: &mut Vec<BulletItem>,
    current_indentation: usize,
) {
    loop {
        if !flat.is_empty() {
            let (indentation, _bullet_data) = &flat[0];
            // copy the data for borrowing rules
            let indentation = *indentation;

            match indentation.cmp(&current_indentation) {
                Ordering::Greater => {
                    let mut new_buffer = Vec::new();
                    collect_bullet_items(flat, &mut new_buffer, indentation);
                    nested.push(BulletItem::Nested(new_buffer));
                }
                Ordering::Less => {
                    // return back to the previous level of indentation
                    return;
                }
                Ordering::Equal => {
                    // we have the same level of indentation
                    let item = flat.remove(0);
                    nested.push(item.1)
                }
            }
        } else {
            break;
        }
    }
}

fn parse_string<'a>(i: &'a str) -> Result<Vec<Span>, NomErr> {
    let span_options = |x: &'a str| {
        alt((
            parse_strikethrough,
            parse_bold,
            parse_italics,
            parse_equation,
            parse_regular_text,
        ))(x)
    };

    let out = nom::multi::many1(span_options)(i)?;

    Ok(out.1)
}

// TODO: does not handle escaped sequences
fn parse_bold(i: &str) -> IResult<&str, Span> {
    let (rest, (_, bolded_text, _)) = tuple((
        //
        tag("**"),
        take_until("**"),
        tag("**"),
    ))(i)?;

    Ok((rest, Span::Bold(bolded_text.to_string())))
}

// TODO: does not handle escaped sequences
fn parse_strikethrough(i: &str) -> IResult<&str, Span> {
    let (rest, (_, strikethrough, _)) = tuple((
        //
        tag("~~"),
        take_until("~~"),
        tag("~~"),
    ))(i)?;

    Ok((rest, Span::Strikethrough(strikethrough.to_string())))
}

// TODO: does not handle escaped sequences
fn parse_italics(i: &str) -> IResult<&str, Span> {
    let (rest, (_, italics, _)) = tuple((
        //
        tag("*"),
        take_until("*"),
        tag("*"),
    ))(i)?;

    Ok((rest, Span::Italics(italics.to_string())))
}

// TODO: does not handle escaped sequences
fn parse_equation(i: &'_ str) -> IResult<&'_ str, Span> {
    let (rest, (_, italics, _)) = tuple((
        //
        tag("$$"),
        take_until("$$"),
        tag("$$"),
    ))(i)?;

    Ok((rest, Span::Equation(italics.to_string())))
}

fn parse_regular_text(i: &'_ str) -> IResult<&'_ str, Span> {
    if i.is_empty() {
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
        if parser(current_slice).is_ok() {
            break;
        }

        idx += 1;

        if let Some(slice) = i.get(idx..) {
            current_slice = slice
        } else {
            if idx >= i.len() {
                // TODO: this might need to be an error - im unsure as of now
                idx -= 1;
                break;
            } else {
                continue;
            }
        }
    }

    Ok((current_slice, i.get(0..idx).unwrap()))
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Block {
    Paragraph(Vec<Span>),
    BulletedList(Vec<BulletItem>),
    Picture(ParsePicture),
    Code(Code),
    Directive(Directive),
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct ParsePicture {
    pub(crate) path: String,
    pub(crate) caption: Option<String>,
    pub(crate) directive: Option<Vec<PictureDirective>>,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum PictureDirective {
    Vertical,
    Width(String),
    Height(String),
}

impl PictureDirective {
    pub(crate) fn is_orientation(&self) -> bool {
        match &self {
            Self::Vertical => true,
            Self::Width(_) | Self::Height(_) => false,
        }
    }

    pub(crate) fn is_width(&self) -> bool {
        match &self {
            Self::Width(_) => true,
            Self::Vertical | Self::Height(_) => false,
        }
    }

    pub(crate) fn is_height(&self) -> bool {
        match &self {
            Self::Height(_) => true,
            Self::Vertical | Self::Width(_) => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Directive {
    NewSlide,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum BulletItem {
    // a single bullet point that is at the same indentation level
    Single(Vec<Span>),
    // a set of bullet points at one more indentation level than the current level
    Nested(Vec<BulletItem>),
}

#[derive(Debug, PartialEq, Clone)]
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
        let (_, first_slide) = out.unwrap();
        let expected_title = vec![Span::Text("Presentation Title".to_string())];

        assert_eq!(first_slide.title, expected_title);
        assert_eq!(first_slide.author, "Author Name");
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

        let expected = vec![
            Block::Paragraph(vec![Span::Text("some inner text".to_string())]),
            Block::Paragraph(vec![Span::Text("some more text".to_string())]),
        ];

        assert_eq!(out.1.contents, expected);
    }

    #[test]
    fn parse_slide_2() {
        let text = r#"
## Energy

```
! from calculate_energy https://github.com/Fluid-Dynamics-Group/hit3d/blob/master/src/write_data.f90#L127
do i =1,nx
	do j=1,ny
		do k=1,nz
			u = wrk(i,j,k,1)
			v = wrk(i,j,k,2)
			w = wrk(i,j,k,3)

			energy = energy + u**2 + v**2 + w**2
		end do
	end do
end do

energy = energy * dx * dy * dz
```

"#;

        let out = parse_slide(text);
        dbg!(&out);
        let out = out.unwrap();

        for c in out.1.contents {
            assert_eq!(matches!(c, Block::Picture(_)), false)
        }
    }

    #[test]
    fn parse_picture_1() {
        let text = "![caption](path)";
        let out = parse_block_as_picture(text);
        dbg!(&out);
        let out = out.unwrap().1;
        let expected = ParsePicture {
            path: "path".to_string(),
            caption: Some("caption".to_string()),
            directive: None,
        };
        assert_eq!(out, Block::Picture(expected));
    }

    #[test]
    fn parse_code_1() {
        let code = r#"
```language
code here
```
        "#;

        let out = parse_as_code(code);
        dbg!(&out);
        let out = out.unwrap().1;
        let expected = Block::Code(Code::new("code here\n".to_string(), "language".to_string()));
        assert_eq!(out, expected);
    }

    #[test]
    fn parse_code_2() {
        let code = r#"

```
! from calculate_energy https://github.com/Fluid-Dynamics-Group/hit3d/blob/master/src/write_data.f90#L127
do i =1,nx
	do j=1,ny
		do k=1,nz
			u = wrk(i,j,k,1)
			v = wrk(i,j,k,2)
			w = wrk(i,j,k,3)

			energy = energy + u**2 + v**2 + w**2
		end do
	end do
end do

energy = energy * dx * dy * dz
```
        "#;

        let out = parse_as_code(code);
        dbg!(&out);
        let out = out.unwrap().1;
        assert_eq!(matches!(out, Block::Code(_)), true);
    }

    #[test]
    fn code_header_1() {
        let text = "```python\n";
        let out = code_block_header(text);
        dbg!(&out);
        let out = out.unwrap();
        assert_eq!(&out.1.language, "python")
    }

    #[test]
    fn slide_with_breaks() {
        let text = "\n\n
            ## Some Slide

            text1

            %NEWSLIDE

            text2";

        let out = parse_slide(text);

        let out = out.unwrap().1;

        assert_eq!(out.title, vec![Span::Text("Some Slide".into())]);

        let expected_blocks = vec![
            Block::Paragraph(vec![Span::Text("text1".into())]),
            Block::Directive(Directive::NewSlide),
            Block::Paragraph(vec![Span::Text("text2".into())]),
        ];

        assert_eq!(out.contents, expected_blocks);
    }

    #[test]
    fn picture_with_directive() {
        let text = "![](somepicture)\n%VERTICAL";
        let out = parse_block_as_picture(text).unwrap();
        let picture = out.1;

        let expected = ParsePicture {
            path: "somepicture".to_string(),
            caption: None,
            directive: Some(vec![PictureDirective::Vertical]),
        };

        assert_eq!(picture, Block::Picture(expected));
    }

    #[test]
    fn non_ascii_characters() {
        let slide = "
            ## Slide Name

            ```
            .
            └── example_namespace

            ```
        ";

        let output = parse_slide(&slide);
        dbg!(&output);

        output.unwrap();
    }

    #[test]
    fn multiple_picture_directives() {
        let text = "%VERTICAL\n%WIDTH=SOMETHING\nREST";

        let first = picture_directive(text);
        dbg!(&first);
        let (rest, dir1) = first.unwrap();

        let second = picture_directive(rest);
        dbg!(&second);
        let (rest, dir2) = second.unwrap();

        assert!(dir1 == PictureDirective::Vertical);
        assert!(dir2 == PictureDirective::Width("SOMETHING".into()));
        assert!(rest == "\nREST");
    }

    #[test]
    fn slide_multiple_directives() {
        let text = "
            ## Slide Name

            ![caption](./image/path)
            %VERTICAL
            %WIDTH=SOMETHING

            REST"
            ;

        let slide = parse_slide(text);
        dbg!(&slide);

        let slide = slide.unwrap();

        dbg!(&slide.1.contents);

        let output = match &slide.1.contents[0] {
            Block::Picture(x) => x,
            _ => panic!("unexpected slide"),
        };

        let expected = ParsePicture {
            path: "./image/path".into(),
            caption: Some("caption".into()),
            directive: Some(vec![
                PictureDirective::Vertical,
                PictureDirective::Width("SOMETHING".into()),
            ]),
        };

        assert_eq!(&expected, output);
    }
}
