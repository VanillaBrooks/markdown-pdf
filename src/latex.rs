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
        r#"\begin{frame}
    \frametitle{"#
            .as_bytes(),
    )?;

    writer.write_all(slide.title.to_latex().as_bytes())?;
    writer.write_all("}\n\n".as_bytes())?;
    // Now finished writing the frame title
    //

    writer.write_all(slide.contents.to_latex().as_bytes())?;

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

    writer.write_all(title.to_latex().as_bytes())?;

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
