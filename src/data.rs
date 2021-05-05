use markdown::Span;
use std::path::PathBuf;

macro_rules! filter_span {
    ($var:ident, $($ext:ident),*) => {
        match $var {
            $( Span::$ext(..) => true,)*
            _ => false
        }

    };
    (@create_match $ext:ident) => {
        Span::$ext => true,
    };
}

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

#[derive(Debug)]
pub(crate) enum ContentOptions {
    OnlyText(Vec<Content>),
    OnlyPicture(Picture),
    TextAndPicture(Vec<Content>, Picture),
}

#[derive(Debug)]
pub(crate) enum Content {
    Text(Vec<Span>),
    BulletList(Vec<markdown::ListItem>),
}

#[derive(Debug)]
pub(crate) enum Picture {
    Path { path: PathBuf, caption: String },
    Link { link: String, caption: String },
}
impl Picture {
    fn get_path(&self) -> PathBuf {
        match &self {
            Picture::Path { path, .. } => path.clone(),
            Picture::Link { .. } => {
                // TODO: download the picture
                panic!("does not currently handle hyperlinks to pictures");
            }
        }
    }

    fn caption(&self) -> &str {
        match &self {
            Picture::Path { caption, .. } => caption,
            Picture::Link { caption, .. } => &caption,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Title {
    title: Vec<Span>,
}

impl From<Vec<Span>> for Title {
    fn from(title: Vec<Span>) -> Self {
        Self { title }
    }
}

pub(crate) trait Latex {
    fn to_latex(self) -> String;
}

impl Latex for Title {
    fn to_latex(self) -> String {
        let mut buffer = String::new();
        text_latex(&mut buffer, self.title);
        buffer
    }
}

impl Latex for ContentOptions {
    fn to_latex(self) -> String {
        let mut buffer = String::new();

        match self {
            ContentOptions::OnlyText(content_list) => {
                for c in content_list {
                    buffer.push_str(&c.to_latex())
                }
            }
            ContentOptions::OnlyPicture(picture) => {
                picture_latex(&mut buffer, picture, false);
            }
            ContentOptions::TextAndPicture(content, picture) => {
                buffer.push_str("\t\\begin{minipage}{0.4\\textwidth}\n");

                for c in content {
                    buffer.push_str(&c.to_latex());
                }

                buffer.push_str("\n\t\\end{minipage}%\n");
                buffer.push_str("\t\\hfill\n");
                buffer.push_str("\t\\begin{minipage}{0.55\\textwidth}\n");

                picture_latex(&mut buffer, picture, true);

                buffer.push_str("\t\\end{minipage}\n");
            }
        }

        buffer
    }
}

impl Latex for Content {
    fn to_latex(self) -> String {
        let mut buffer = String::new();
        match self {
            Content::Text(spans) => text_latex(&mut buffer, spans),
            Content::BulletList(span_spans) => bulletpoints_latex(&mut buffer, span_spans),
        }
        buffer
    }
}

fn picture_latex(buffer: &mut String, picture: Picture, is_split: bool) {
    let path = picture.get_path();

    buffer.push_str(
        r#"
    \begin{figure}
        \centering"#,
    );

    if is_split {
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

    buffer.push_str(
        &path
            .into_os_string()
            .into_string()
            .expect("path provided was not utf8"),
    );
    buffer.push_str(
        r#"}
        \caption{"#,
    );
    buffer.push_str(picture.caption());
    buffer.push_str(
        r#"}
    \end{figure}"#,
    );
}

fn bulletpoints_latex(buffer: &mut String, spans: Vec<markdown::ListItem>) {
    buffer.push_str("\t\\begin{itemize}\n");

    for item in spans {
        match item {
            markdown::ListItem::Simple(span) => {
                //
                buffer.push_str("\t\t\\item ");
                text_latex(buffer, span);
                buffer.push_str("\n");
            }
            markdown::ListItem::Paragraph(paragraph) => {
                for block in paragraph {
                    match block {
                        markdown::Block::Paragraph(new_list) => {
                            buffer.push_str("\t\t\\item ");
                            text_latex(buffer, new_list);
                            buffer.push_str("\n");
                        }
                        markdown::Block::UnorderedList(new_list) => {
                            bulletpoints_latex(buffer, new_list)
                        }
                        _ => (),
                    }
                }

                //
            }
        }
    }

    buffer.push_str("\t\\end{itemize}\n");
}
fn text_latex(buffer: &mut String, spans: Vec<Span>) {
    for i in spans
        .into_iter()
        .filter(|x| filter_span!(x, Text, Emphasis, Strong))
    {
        match i {
            Span::Text(text) => buffer.push_str(&text),
            Span::Emphasis(new_spans) => {
                buffer.push_str("\\emph{");
                text_latex(buffer, new_spans);
                buffer.push('}');
            }
            Span::Strong(new_spans) => {
                buffer.push_str("\\textbf{");
                text_latex(buffer, new_spans);
                buffer.push('}');
            }
            _ => unreachable!(),
        }

        //
    }
}
