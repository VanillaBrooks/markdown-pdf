use super::data::{Content, ContentOptions, Picture, Presentation, Slide, Title};
use super::Error;
use std::io::Read;
use std::iter::Peekable;
use std::path::PathBuf;

use markdown::{Block, Span};

pub(crate) fn parse_markdown<R: Read>(mut reader: R) -> Result<Presentation, Error> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let text = String::from_utf8(buffer)?;
    let tokens = markdown::tokenize(&text).into_iter();

    let (title, author, tokens) = read_title(tokens)?;

    let mut tokens = tokens.peekable();

    let mut slides = vec![];

    while tokens.len() != 0 {
        let (slide, tmp) = read_slide(tokens)?;
        tokens = tmp;
        slides.push(slide);
    }

    Ok(Presentation {
        title,
        author,
        slides,
    })
}

fn read_title<I: Iterator<Item = markdown::Block>>(
    mut iter: I,
) -> Result<(Title, String, I), MarkdownError> {
    let raw_title = iter.next().ok_or(MarkdownError::MissingTitle)?;
    let author_section = iter.next().ok_or(MarkdownError::MissingAuthor)?;

    const HEADER_STRING: &'static str = "AUTHOR=";

    let author = if let Block::Paragraph(mut para) = author_section {
        let author_span = para.remove(0);
        if let Span::Text(author_string) = author_span {
            if author_string.starts_with(HEADER_STRING) {
                author_string
                    .get(HEADER_STRING.len()..)
                    .expect("You must specify an AUTHOR=your name here")
                    .to_string()
            } else {
                println!("WARN: Missing author (use AUTHOR=... under your # title)");
                "".to_string()
            }
        } else {
            println!("WARN: Missing author (use AUTHOR=... under your # title)");
            "".to_string()
        }
    } else {
        "".to_string()
    };

    Ok((read_header(raw_title)?, author, iter))
}

struct PictureContainer {
    picture: Option<Picture>,
}
impl PictureContainer {
    fn push(&mut self, url_or_path: String, caption: String) {
        let picture = if url_or_path.starts_with("http") {
            Picture::Link {
                link: url_or_path,
                caption,
            }
        } else {
            Picture::Path {
                path: PathBuf::from(url_or_path),
                caption,
            }
        };
        self.picture = Some(picture);
    }
}

struct ContentContainer {
    content: Option<Vec<Content>>,
}
impl ContentContainer {
    fn push(&mut self, content: Content) {
        if let Some(c) = &mut self.content {
            c.push(content)
        } else {
            self.content = Some(vec![content])
        }
    }
}

fn read_slide<I: Iterator<Item = markdown::Block>>(
    mut iter: Peekable<I>,
) -> Result<(Slide, Peekable<I>), MarkdownError> {
    let raw_title = iter.next().ok_or(MarkdownError::MissingTitle)?;

    let title = read_header(raw_title)?;

    let mut content_container = ContentContainer { content: None };
    let mut picture_container = PictureContainer { picture: None };

    loop {
        let opt_peek = iter.peek();

        // check if the next item is a header (to start a new slide) or even exists
        if let Some(peek) = opt_peek {
            if is_header(peek) {
                break;
            }
        } else {
            break;
        }

        // unwrap is fine here since we already checked the .peek()
        let item = iter.next().unwrap();

        match item {
            Block::Header(_, _) => unreachable!(),
            Block::Paragraph(spans) => {
                //
                let (mut picture_vec, content_vec): (Vec<_>, Vec<_>) = spans
                    .into_iter()
                    .partition(|x| matches!(x, Span::Image { .. }));

                if content_vec.len() > 0 {
                    content_container.push(Content::Text(content_vec));
                }

                if picture_vec.len() != 0 {
                    let picture = picture_vec.remove(picture_vec.len() - 1);
                    if let Span::Image(caption, url_path, _) = picture {
                        picture_container.push(url_path, caption)
                    }
                }
            }
            Block::Blockquote(_) => println!("WARN: Blockquotes are currently unhandled"),
            Block::OrderedList(_, _) => println!("WARN: ordered lists are currently unhandled"),
            Block::CodeBlock(_, _) => println!("WARN: Code blocks are currently unhandled"),
            Block::UnorderedList(list) => {
                content_container.push(Content::BulletList(list));
            }
            Block::Raw(_) => println!("WARN: raw items unhandled"),
            Block::Hr => (),
        }
    }
    let contents = match (picture_container.picture, content_container.content) {
        (Some(picture), Some(content)) => ContentOptions::TextAndPicture(content, picture),
        (Some(picture), None) => ContentOptions::OnlyPicture(picture),
        (None, Some(content)) => ContentOptions::OnlyText(content),
        (None, None) => ContentOptions::OnlyText(vec![Content::Text(vec![Span::Text("".into())])]),
    };

    let slide = Slide { title, contents };

    Ok((slide, iter))
}

fn read_header(item: Block) -> Result<Title, MarkdownError> {
    let title = if let Block::Header(span, _size) = item {
        span
    } else {
        return Err(MarkdownError::MissingTitle);
    };

    Ok(title.into())
}

fn is_header(item: &Block) -> bool {
    if let Block::Header { .. } = item {
        true
    } else {
        false
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MarkdownError {
    #[error("There was no `# title` or `## title` defined for the markdown document")]
    MissingTitle,
    #[error("There was no author specified on the document")]
    MissingAuthor,
}
