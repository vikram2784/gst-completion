mod gstreamer;

use nom::branch::alt;
use nom::bytes::complete::{is_a, is_not, take};
use nom::character::complete::{char, space0, space1};
use nom::combinator::{map_res, not, opt, peek};
use nom::multi::{many0, many1};
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
use nom::{IResult};

use shell_completion::{BashCompletionInput, CompletionInput, CompletionSet};

fn parse(
    s: &str,
) -> (
    i8,
    IResult<&str, Vec<(&str, Vec<(&str, &str)>, Option<(Option<&str>, &str)>)>>,
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
        opt(preceded(
            peek(not(char('!'))),
            separated_pair(
                opt(is_not(".")),
                char('.'),
                terminated(is_not(" \t"), space0),
            ),
        )),
    )))(s);

    (index, res)
}

fn is_remainder_sane(_input: &BashCompletionInput, rem: &str) -> bool {
    let res: IResult<&str, &str> = preceded(char('!'), space1)(rem);
    if let Ok(("", _)) = res {
        true
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

    if !input
        .current_word()
        .chars()
        .all(|x| x.is_alphanumeric() || x == '_' || x == '-')
    {
        return;
    }

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

            let (prev_elem, prev_pad) = if let Some(pad) = prev.2 {
                if let Some(elem_name) = pad.0 {
                    let found = parsed.iter().find(|elem| {
                        elem.1
                            .iter()
                            .find(|prop| prop.0 == "name" && prop.1 == elem_name)
                            .is_some()
                    });

                    if let Some(elem) = found {
                        (elem.0, Some(pad.1))
                    } else {
                        ("", Some(pad.1))
                    }
                } else {
                    (parsed[index].0, Some(pad.1))
                }
            } else {
                (parsed[index].0, None)
            };

            if let Some(element) = gstreamer::find_element(prev_elem, prev_pad) {
                element.get_compatible_elements(current_word).suggest();
            }
        } else if let Some(element) = gstreamer::find_element(parsed[i as usize].0, None)  {
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

/*
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
            (1, Ok(("", vec![("filesrc", vec![])])))
        );
    }

    #[test]
    fn test2() {
        assert_eq!(
            parse("! filesrc ! fakesink "),
            (1, Ok(("", vec![("filesrc", vec![]), ("fakesink", vec![])])))
        );
    }

    #[test]
    fn test3() {
        assert_eq!(
            parse("! filesrc ! fakesink ! "),
            (
                2,
                Ok(("! ", vec![("filesrc", vec![]), ("fakesink", vec![])]))
            )
        );
    }

    #[test]
    fn test4() {
        assert_eq!(
            parse("! filesrc ! fakesink !"),
            (
                1,
                Ok(("!", vec![("filesrc", vec![]), ("fakesink", vec![])]))
            )
        );
    }

    #[test]
    fn test5() {
        assert_eq!(
            parse("! fakesrc ! fakesink name = test   !    "),
            (
                2,
                Ok((
                    "!    ",
                    vec![("fakesrc", vec![]), ("fakesink", vec![("name", "test")])]
                ))
            )
        );
    }

    #[test]
    fn test6() {
        assert_eq!(
            parse("! fakesrc ! identity name = test  !   fakesi "),
            (
                2,
                Ok((
                    "",
                    vec![
                        ("fakesrc", vec![]),
                        ("identity", vec![("name", "test")]),
                        ("fakesi", vec![])
                    ]
                ))
            )
        );
    }

    #[test]
    fn test7() {
        assert_eq!(
            parse("! fakesrc ! identity name = test  !   fakesink name   "),
            (
                2,
                Ok((
                    "name   ",
                    vec![
                        ("fakesrc", vec![]),
                        ("identity", vec![("name", "test")]),
                        ("fakesink", vec![])
                    ]
                ))
            )
        );
    }

    #[test]
    fn test8() {
        assert_eq!(
            parse("! fakesrc ! identity name = test  !   fakesink name =  "),
            (
                2,
                Ok((
                    "name =  ",
                    vec![
                        ("fakesrc", vec![]),
                        ("identity", vec![("name", "test")]),
                        ("fakesink", vec![])
                    ]
                ))
            )
        );
    }

    #[test]
    fn test9() {
        assert_eq!(
            parse("! fakesrc ! identity name = test  !   fakesink name = prop = 1"),
            (
                2,
                Ok((
                    "= 1",
                    vec![
                        ("fakesrc", vec![]),
                        ("identity", vec![("name", "test")]),
                        ("fakesink", vec![("name", "prop")])
                    ]
                ))
            )
        );
    }

    #[test]
    fn test10() {
        assert_eq!(
            parse("! fakesrc ! identity name = test  !   fakesink name = s -e prop v = 1"),
            (
                2,
                Ok((
                    "prop v = 1",
                    vec![
                        ("fakesrc", vec![]),
                        ("identity", vec![("name", "test")]),
                        ("fakesink", vec![("name", "s")])
                    ]
                ))
            )
        );
    }

    #[test]
    fn test11() {
        assert_eq!(
            parse("! fakesrc ! identity name = test  !   fakesink name = s prop v = 1 ! abc "),
            (
                2,
                Ok((
                    "prop v = 1 ! abc ",
                    vec![
                        ("fakesrc", vec![]),
                        ("identity", vec![("name", "test")]),
                        ("fakesink", vec![("name", "s")])
                    ]
                ))
            )
        );
    }

    #[test]
    fn test12() {
        assert_eq!(
            parse("! filesrc ! fakesink"),
            (1, Ok(("", vec![("filesrc", vec![]), ("fakesink", vec![])])))
        );
    }
}
*/
