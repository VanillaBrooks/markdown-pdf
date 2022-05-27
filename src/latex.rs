use super::data::{Latex, Presentation, Slide, Title};
use super::Error;
use std::io::Write;

pub(crate) fn write_latex<W: Write>(
    mut writer: W,
    presentation: Presentation,
) -> Result<(), Error> {
    latex_header(&mut writer, presentation.title, presentation.author)?;

    for slide in presentation.slides {
        write_slide(&mut writer, slide)?;
    }

    latex_footer(&mut writer)?;

    Ok(())
}

fn write_slide<W: Write>(mut writer: W, slide: Slide) -> Result<(), Error> {
    writer.write_all(
        r#"\begin{frame}[fragile]
    \frametitle{"#
            .as_bytes(),
    )?;

    let mut buffer = String::with_capacity(200);
    slide.title.to_latex(&mut buffer);
    writer.write_all(buffer.as_bytes())?;
    writer.write_all("}\n\n".as_bytes())?;

    // Now finished writing the frame title

    let mut buffer = String::with_capacity(200);
    slide.contents.to_latex(&mut buffer);

    writer.write_all(buffer.as_bytes())?;

    writer.write_all("\n\\end{frame}\n\n\n".as_bytes())?;

    Ok(())
}

fn latex_header<W: Write>(mut writer: W, title: Title, author: String) -> Result<(), Error> {
    writer.write_all(
        r#"
\documentclass{beamer}
\usepackage{graphicx}
\usepackage{float}
\usepackage{hyperref}
\usepackage{ulem}
\usepackage{listings}
\usepackage{xcolor}
\usepackage{esint}
\usepackage{gensymb}
\usepackage{mathtools}
\usepackage{multirow}

\definecolor{codegreen}{rgb}{0,0.6,0}
\definecolor{codegray}{rgb}{0.5,0.5,0.5}
\definecolor{codepurple}{rgb}{0.58,0,0.82}
\definecolor{backcolour}{rgb}{0.95,0.95,0.92}

\lstdefinestyle{mystyle}{
    backgroundcolor=\color{backcolour},
    commentstyle=\color{codegreen},
    keywordstyle=\color{magenta},
    numberstyle=\tiny\color{codegray},
    stringstyle=\color{codepurple},
    basicstyle=\ttfamily\tiny,
    breakatwhitespace=false,
    breaklines=true,
    captionpos=b,
    keepspaces=true,
    numbers=left,
    numbersep=5pt,
    showspaces=false,
    showstringspaces=false,
    showtabs=false,
    tabsize=2
}

\lstset{style=mystyle}

\hypersetup{
colorlinks=true,
linkcolor=blue,
filecolor=magenta,
urlcolor=cyan,
}

\urlstyle{same}

\title{"#
            .as_bytes(),
    )?;

    let mut buffer = String::with_capacity(200);
    title.to_latex(&mut buffer);
    writer.write_all(buffer.as_bytes())?;

    writer.write_all(
        r#"}
\date{\today}
\author{"#
            .as_bytes(),
    )?;

    writer.write_all(author.as_bytes())?;

    writer.write_all(
        r#"}
\begin{document}
\frame{\titlepage}

"#
        .as_bytes(),
    )?;

    Ok(())
}

fn latex_footer<W: Write>(mut writer: W) -> Result<(), Error> {
    writer.write_all(
        r#"
\end{document}"#
            .as_bytes(),
    )?;
    Ok(())
}
