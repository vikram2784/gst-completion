mod gstreamer;

use nom::branch::alt;
use nom::bytes::complete::{is_a, is_not, take};
use nom::character::complete::{char, space0, space1};
use nom::combinator::{map_res, not, opt, peek, rest};
use nom::multi::{many0, many1};
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
use nom::IResult;

use shell_completion::{BashCompletionInput, CompletionInput, CompletionSet};

fn parse(
    s: &str,
) -> (
    i8,
    IResult<
        &str,
        Vec<(
            &str,
            Vec<(&str, &str)>,
            Option<(Option<&str>, Option<&str>)>,
        )>,
    >,
) {
    let mut index = -1;

    let res = many1(tuple((
        delimited(
            map_res(
                tuple((
                    char('!'),
                    space1,
                    many0(tuple((char('-'), is_not(" \t"), space0))),
                )),
                |x| {
                    index = index + 1;
                    Ok::<_, nom::Err<&str>>(x)
                },
            ),
            map_res(is_not(" \t"), |s: &str| {
                if s.chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    Ok(s)
                } else {
                    Err(nom::Err::<&str>::Error("error"))
                }
            }),
            tuple((space0, many0(tuple((char('-'), is_not(" \t"), space0))))),
        ),
        terminated(
            many0(separated_pair(
                preceded(
                    peek(not(char('!'))),
                    map_res(is_not("= \t"), |s: &str| {
                        if s.chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                        {
                            Ok(s)
                        } else {
                            Err(nom::Err::<&str>::Error("error"))
                        }
                    }),
                ),
                tuple((space0, char('='), space0)),
                alt((
                    delimited(
                        is_a("\"\'"),
                        is_not("\"\'"),
                        tuple((
                            take(1u8),
                            space1,
                            many0(tuple((char('-'), is_not(" \t"), space0))),
                        )),
                    ),
                    map_res(
                        terminated(
                            terminated(is_not(" \t"), space1),
                            many0(tuple((char('-'), is_not(" \t"), space0))),
                        ),
                        |s: &str| {
                            if s.contains("\'") || s.contains("\"") {
                                Err(nom::Err::<&str>::Error("error"))
                            } else {
                                Ok(s)
                            }
                        },
                    ),
                )),
            )),
            many0(tuple((char('-'), is_not(" \t"), space0))),
        ),
        map_res(
            terminated(
                opt(preceded(
                    peek(not(char('!'))),
                    separated_pair(opt(is_not(" \t=.")), char('.'), opt(is_not(" \t"))),
                )),
                space0,
            ),
            |x| {
                if let Some(elem_pad) = x {
                    if elem_pad.0.is_none() && elem_pad.1.is_none() {
                        Err(nom::Err::<&str>::Error("error"))
                    } else {
                        Ok(x)
                    }
                } else {
                    Ok(x)
                }
            },
        ),
    )))(s);

    (index, res)
}

fn can_complete_path(rem: &str) -> bool {
    let res: IResult<&str, (&str, Option<&str>)> = separated_pair(
        terminated(is_not("= \t"), space0),
        tuple((char('='), space0)),
        opt(is_not(" \t")),
    )(rem);

    if let Ok((o, (_, _))) = res {
        o.is_empty()
    } else {
        false
    }
}

fn is_remainder_sane(input: &BashCompletionInput, rem: &str) -> bool {
    let res: IResult<&str, &str> = preceded(char('!'), space1)(rem);
    if let Ok(("", _)) = res {
        true
    } else if can_complete_path(rem) {
        input.complete_file().suggest();
        false
    } else if rem
        .chars()
        .all(|x| x.is_alphanumeric() || x == '-' || x == '_')
    {
        true
    } else {
        false
    }
}

fn main() {
    gstreamer::init();

    let input = BashCompletionInput::from_env().expect("Missing expected environment variables");

    let current_word = if input.current_word().is_empty() {
        None
    } else {
        Some(input.current_word())
    };

    let args = {
        let mut c = input.args();
        c[0] = "!";
        c
    }
    .join(" ");

    if let (i, Ok((rem, parsed))) = parse(&args) {
        if !is_remainder_sane(&input, rem) {
            return;
        }

        let len = parsed.len();
        assert!(len > 0);

        if len as i8 == i
            || (parsed[i as usize].1.len() == 0 && current_word == Some(parsed[i as usize].0))
        {
            if i == 0 {
                return gstreamer::get_elements(current_word).suggest();
            }

            let index = (i - 1) as usize;
            let prev = &parsed[index];

            let (prev_elem, prev_pad) = if let Some(elem_pad) = prev.2 {
                match elem_pad.0 {
                    Some(elem_name) => {
                        let found = parsed.iter().find(|elem| {
                            elem.1
                                .iter()
                                .find(|prop| prop.0 == "name" && prop.1 == elem_name)
                                .is_some()
                        });

                        if let Some(elem) = found {
                            (elem.0, elem_pad.1)
                        } else {
                            ("", elem_pad.1)
                        }
                    }
                    _ => (parsed[index].0, elem_pad.1),
                }
            } else {
                (parsed[index].0, None)
            };

            if let Some(element) = gstreamer::find_element(prev_elem, prev_pad) {
                element.get_compatible_elements(current_word).suggest();
            }
        } else if let Some(element) = gstreamer::find_element(parsed[i as usize].0, None) {
            if parsed[i as usize].2.is_some() {
                return;
            }

            let arr = parsed[i as usize]
                .1
                .iter()
                .map(|x| x.0)
                .collect::<Vec<&str>>();

            element.get_property_names(&arr, current_word).suggest();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test0() {
        assert_eq!(
            parse("! "),
            (
                0i8,
                Err(nom::Err::Error(nom::error::Error {
                    input: "",
                    code: nom::error::ErrorKind::IsNot
                }))
            )
        );
    }

    #[test]
    fn test1() {
        assert_eq!(
            parse("! filesrc "),
            (1, Ok(("", vec![("filesrc", vec![], None)])))
        );
    }

    #[test]
    fn test2() {
        assert_eq!(
            parse("! filesrc ! fakesink "),
            (
                1,
                Ok((
                    "",
                    vec![("filesrc", vec![], None), ("fakesink", vec![], None)]
                ))
            )
        );
    }

    #[test]
    fn test3() {
        assert_eq!(
            parse("! filesrc ! fakesink ! "),
            (
                2,
                Ok((
                    "! ",
                    vec![("filesrc", vec![], None), ("fakesink", vec![], None)]
                ))
            )
        );
    }

    #[test]
    fn test4() {
        assert_eq!(
            parse("! filesrc ! fakesink !"),
            (
                1,
                Ok((
                    "!",
                    vec![("filesrc", vec![], None), ("fakesink", vec![], None)]
                ))
            )
        );
    }

    #[test]
    fn test5() {
        assert_eq!(
            parse("! filesrc ! fakesink name=abc test="),
            (
                1,
                Ok((
                    "test=",
                    vec![
                        ("filesrc", vec![], None),
                        ("fakesink", vec![("name", "abc")], None)
                    ]
                ))
            )
        );
    }

    #[test]
    fn test6() {
        assert_eq!(
            parse("! filesrc ! fakesink name=abc test =   "),
            (
                1,
                Ok((
                    "test =   ",
                    vec![
                        ("filesrc", vec![], None),
                        ("fakesink", vec![("name", "abc")], None)
                    ]
                ))
            )
        );
    }

    #[test]
    fn test7() {
        assert_eq!(
            parse("! filesrc ! fakesink name=abc test = \" random=  "),
            (
                1,
                Ok((
                    "test = \" random=  ",
                    vec![
                        ("filesrc", vec![], None),
                        ("fakesink", vec![("name", "abc")], None)
                    ]
                ))
            )
        );
    }

    #[test]
    fn test8() {
        assert_eq!(
            parse("! filesrc ! fakesink name=abc test =  random=  "),
            (
                1,
                Ok((
                    "",
                    vec![
                        ("filesrc", vec![], None),
                        ("fakesink", vec![("name", "abc"), ("test", "random=")], None)
                    ]
                ))
            )
        );
    }

    #[test]
    fn test9() {
        assert_eq!(
            parse("! filesrc ! fakesink name=abc test =  random=  "),
            (
                1,
                Ok((
                    "",
                    vec![
                        ("filesrc", vec![], None),
                        ("fakesink", vec![("name", "abc"), ("test", "random=")], None)
                    ]
                ))
            )
        );
    }
}
