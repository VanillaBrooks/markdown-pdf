use super::parse::{Block, BulletItem, Span};

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

#[derive(Debug, PartialEq)]
pub(crate) enum ContentOptions {
    OnlyText(Vec<Block>),
    OnlyPicture(Picture),
    TextAndPicture(Vec<Block>, Picture),
}

#[derive(Debug, PartialEq)]
pub(crate) enum Picture {
    Path { path: String, caption: String },
    Link { link: String, caption: String },
}
impl Picture {
    fn to_latex_picture(&self, is_split: bool) -> LatexPicture {
        LatexPicture {
            picture: &self,
            is_split,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct LatexPicture<'a> {
    picture: &'a Picture,
    is_split: bool,
}

impl<'a> LatexPicture<'a> {
    fn get_path(&self) -> String {
        match &self.picture {
            Picture::Path { path, .. } => path.clone(),
            Picture::Link { .. } => {
                // TODO: download the picture
                panic!("does not currently handle hyperlinks to pictures");
            }
        }
    }

    fn caption(&self) -> &str {
        match &self.picture {
            Picture::Path { caption, .. } => caption,
            Picture::Link { caption, .. } => &caption,
        }
    }
}

#[derive(Debug, PartialEq)]
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

        if self.is_split {
            buffer.push_str(
                r#"
        \includegraphics[width=\textwidth]{"#,
            );
        } else {
            buffer.push_str(
                r#"
        \includegraphics[width=0.9\paperwidth,height=0.7\paperheight,keepaspectratio]{"#,
            );
        }

        buffer.push_str(&path);
        buffer.push_str(
            r#"}
        \caption{"#,
        );
        buffer.push_str(self.caption());
        buffer.push_str(
            r#"}
    \end{figure}"#,
        );
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
            ContentOptions::TextAndPicture(content, picture) => {
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
