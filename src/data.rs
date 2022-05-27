use super::parse::{Block, BulletItem, ParsePicture, PictureDirective, Span};


#[derive(Debug)]
pub(crate) struct Presentation {
    pub(crate) title: Title,
    pub(crate) author: String,
    //author: String,
    pub(crate) slides: Vec<Slide>,
}

#[derive(Debug)]
pub(crate) struct Slide {
    pub(crate) title: Title,
    pub(crate) contents: ContentOptions,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum ContentOptions {
    OnlyText(Vec<Block>),
    OnlyPicture(Picture),
    TextAndPicture(Vec<Block>, Picture),
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Picture {
    path: String,
    caption: Option<String>,
    orientation: Orientation,
    width: Option<String>,
    height: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
enum Orientation {
    Vertical,
    Horizonal,
}

impl Picture {
    fn to_latex_picture(&self, is_split: bool) -> LatexPicture {
        LatexPicture {
            picture: &self,
            is_split,
            width: self
                .width
                .as_ref().map(|x| x.as_str()),
            // TODO: might need to fix this for split middle
            height: self
                .width
                .as_ref().map(|x| x.as_str()),
        }
    }
}

impl From<ParsePicture> for Picture {
    fn from(x: ParsePicture) -> Self {
        let ParsePicture {
            path,
            caption,
            directive,
        } = x;

        let (width, height, orientation) = if let Some(directives) = directive {
            let width: Option<String> = directives.iter().find_map(|x| match x {
                PictureDirective::Vertical => None,
                PictureDirective::Width(x) => Some(x.clone()),
                PictureDirective::Height(_) => None,
            });

            let height: Option<String> = directives.iter().find_map(|x| match x {
                PictureDirective::Vertical => None,
                PictureDirective::Width(_) => None,
                PictureDirective::Height(x) => Some(x.clone()),
            });

            let orientation: Orientation = directives
                .iter()
                .find_map(|x| match x {
                    PictureDirective::Vertical => Some(Orientation::Vertical),
                    PictureDirective::Width(_) => None,
                    PictureDirective::Height(_) => None,
                })
                .unwrap_or(Orientation::Horizonal);

            (width, height, orientation)
        } else {
            (None, None, Orientation::Horizonal)
        };

        Self {
            path,
            caption,
            width,
            height,
            orientation,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Code {
    language: String,
    text: String,
}

impl Code {
    pub(crate) fn new(text: String, language: String) -> Self {
        Code { language, text }
    }
}

impl<'a> Latex for Code {
    fn to_latex(&self, buffer: &mut String) {
        buffer.push_str(r#"\begin{lstlisting}[language="#);
        buffer.push_str(&self.language);
        buffer.push_str("]\n");

        buffer.push_str(&self.text);

        buffer.push_str("\\end{lstlisting}")
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct LatexPicture<'a> {
    picture: &'a Picture,
    is_split: bool,
    width: Option<&'a str>,
    height: Option<&'a str>,
}

impl<'a> LatexPicture<'a> {
    fn get_path(&self) -> String {
        self.picture.path.clone()
    }

    fn caption(&self) -> Option<&str> {
        self.picture.caption.as_ref().map(|x| x.as_str())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Title {
    pub(crate) title: Vec<Span>,
}

impl From<Vec<Span>> for Title {
    fn from(title: Vec<Span>) -> Self {
        Self { title }
    }
}

pub(crate) trait Latex {
    fn to_latex(&self, buffer: &mut String);
}

impl Latex for Title {
    fn to_latex(&self, buffer: &mut String) {
        self.title.to_latex(buffer);
    }
}

impl<'a> Latex for LatexPicture<'a> {
    fn to_latex(&self, buffer: &mut String) {
        let path = self.get_path();
        buffer.push_str(
            r#"
    \begin{figure}
        \centering"#,
        );

        let make_include_graphics = |buffer:&mut String, height: &str, width: &str| {
            buffer.push_str("\n\\includegraphics[");
            buffer.push_str(width);
            buffer.push_str(",");
            buffer.push_str(height);
            buffer.push_str("]{");
        };

        if self.is_split {
            //
            // DEFAULTS FOR THIS SECTION
            //

            let width = if let Some(width) = self.width {
                format!("width={}", width)
            } else {
                format!("width={}", "\\textwidth")
            };

            let height = if let Some(height) = self.height {
                format!("height={}", height)
            } else {
                "".to_string()
            };

            make_include_graphics(buffer, &height, &width);

        } else {
            //
            // DEFAULTS FOR THIS SECTION
            //

            let width = if let Some(width) = self.width {
                format!("width={}", width)
            } else {
                format!("width={}", "0.9\\paperwidth")
            };

            let height = if let Some(height) = self.height {
                format!("height={}", height)
            } else {
                format!("height={}", "0.7\\paperheight")
            };

            make_include_graphics(buffer, &height, &width);
        }

        buffer.push_str(&path);
        buffer.push_str(r#"}"#);

        // only add a caption if we have one
        if let Some(caption) = self.caption() {
            buffer.push_str(r#"\caption{"#);
            buffer.push_str(caption);
            buffer.push_str(r#"}"#);
        }

        buffer.push_str(r#"\end{figure}"#)
    }
}

impl Latex for ContentOptions {
    fn to_latex(&self, buffer: &mut String) {
        match self {
            ContentOptions::OnlyText(content_list) => {
                for c in content_list {
                    c.to_latex(buffer);
                    buffer.push_str("\n\n")
                }
            }
            ContentOptions::OnlyPicture(picture) => {
                picture.to_latex_picture(false).to_latex(buffer);
            }
            ContentOptions::TextAndPicture(content, picture) => match picture.orientation {
                Orientation::Vertical => {
                    for c in content {
                        c.to_latex(buffer)
                    }

                    picture.to_latex_picture(false).to_latex(buffer);
                }
                Orientation::Horizonal => {
                    buffer.push_str("\t\\begin{minipage}{0.4\\textwidth}\n");

                    for c in content {
                        c.to_latex(buffer)
                    }

                    buffer.push_str("\n\t\\end{minipage}%\n");
                    buffer.push_str("\t\\hfill\n");
                    buffer.push_str("\t\\begin{minipage}{0.55\\textwidth}\n");

                    picture.to_latex_picture(true).to_latex(buffer);

                    buffer.push_str("\t\\end{minipage}\n");
                }
            },
        }
    }
}

impl Latex for Block {
    fn to_latex(&self, buffer: &mut String) {
        match self {
            Block::Paragraph(spans) => spans.to_latex(buffer),
            Block::BulletedList(span_spans) => span_spans.to_latex(buffer),
            Block::Picture(_) => {
                panic!("pictures should be removed from blocks prior to processing")
            }
            Block::Directive(_) => {
                panic!("directives should have been remove from the presentation as a postprocess step")
            }
            Block::Code(code) => code.to_latex(buffer),
        }
    }
}

impl Latex for Vec<BulletItem> {
    fn to_latex(&self, buffer: &mut String) {
        buffer.push_str("\n\\begin{itemize}\n");

        for item in self.iter() {
            item.to_latex(buffer)
        }

        buffer.push_str("\n\\end{itemize}\n");
    }
}

impl Latex for BulletItem {
    fn to_latex(&self, buffer: &mut String) {
        match self {
            BulletItem::Single(spans) => {
                buffer.push_str("\\item ");
                spans.to_latex(buffer);
                buffer.push('\n');
            }
            BulletItem::Nested(item_list) => item_list.to_latex(buffer),
        }
    }
}

impl Latex for Vec<Span> {
    fn to_latex(&self, buffer: &mut String) {
        for span in self.iter() {
            span.to_latex(buffer);
        }
    }
}

impl Latex for Span {
    fn to_latex(&self, buffer: &mut String) {
        match self {
            Span::Bold(s) => wrap_text(buffer, "\\textbf{", &s, "}"),
            Span::Strikethrough(s) => wrap_text(buffer, "\\sout{", &s, "}"),
            Span::Italics(s) => wrap_text(buffer, "\\emph{", &s, "}"),
            Span::Text(s) => wrap_text(buffer, "", &s, ""),
            Span::Equation(s) => wrap_text(buffer, "$$", &s, "$$"),
        }
    }
}

fn wrap_text(buffer: &mut String, start: &'static str, inner: &str, end: &'static str) {
    buffer.push_str(start);
    buffer.push_str(inner);
    buffer.push_str(end);
}
