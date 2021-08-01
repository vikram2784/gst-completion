use gst;
use gst::prelude::*;
use gst::{Caps, Element, ElementFactory};
use lazy_static::lazy_static;

lazy_static! {
    static ref LIST: Vec<ElementFactory> = {
        gst::init().unwrap();
        gst::ElementFactory::list_get_elements(
            gst::ElementFactoryListType::ANY,
            gst::Rank::__Unknown(0),
        )
    };
}

pub struct BashGstElement {
    factory: ElementFactory,
    element: Element,
    caps: Caps,
}

impl BashGstElement {
    pub fn get_property_names(&self, filter: &[&str], prefix: Option<&str>) -> Vec<String> {
        self.element
            .list_properties()
            .into_iter()
            .filter_map(|x| {
                let name = x.name();

                match if let Some(p) = prefix {
                    if name.starts_with(p) {
                        &name
                    } else {
                        ""
                    }
                } else {
                    &name
                } {
                    "" => None,
                    x if !filter.contains(&x) => Some(x.to_owned()),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn get_compatible_elements(&self, prefix: Option<&str>) -> Vec<String> {
        let mut v =
            gst::ElementFactory::list_filter(&LIST, &self.caps, gst::PadDirection::Sink, false);

        if let Some(p) = prefix {
            v = v
                .into_iter()
                .filter(|x| x.name().starts_with(p))
                .collect::<Vec<ElementFactory>>();
        }

        v.sort_by(|a, b| {
            let anycaps = gst::Caps::new_any();

            if a.can_sink_all_caps(&anycaps) {
                return std::cmp::Ordering::Greater;
            } else if b.can_sink_all_caps(&anycaps) {
                return std::cmp::Ordering::Less;
            }

            let acaps = a
                .static_pad_templates()
                .iter()
                .filter(|x| x.direction() == gst::PadDirection::Sink)
                .fold(gst::Caps::new_empty(), |mut x, y| {
                    x.merge(y.caps());
                    x
                });

            let bcaps = b
                .static_pad_templates()
                .iter()
                .filter(|x| x.direction() == gst::PadDirection::Sink)
                .fold(gst::Caps::new_empty(), |mut x, y| {
                    x.merge(y.caps());
                    x
                });

            if acaps.is_subset(&bcaps) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });

        v.into_iter().map(|x| x.name().to_string()).collect()
    }
}

fn get_src_caps(factory: &ElementFactory, pad: Option<&str>) -> Caps {
    factory
        .static_pad_templates()
        .into_iter()
        .filter(|x| {
            if x.direction() == gst::PadDirection::Src {
                if let Some(p) = pad {
                    p.starts_with(x.name_template().split("%").nth(0).unwrap())
                } else {
                    true
                }
            } else {
                false
            }
        })
        .fold(gst::Caps::new_empty(), |mut x, y| {
            x.merge(y.caps());
            x
        })
}

pub fn get_elements(prefix: Option<&str>) -> Vec<String> {
    LIST.iter()
        .filter_map(|x| {
            let name = x.name().to_string();

            if let Some(p) = prefix {
                if name.starts_with(p) {
                    Some(name)
                } else {
                    None
                }
            } else {
                Some(name)
            }
        })
        .collect()
}

pub fn find_element(name: &str, pad: Option<&str>) -> Option<BashGstElement> {
    if let Some(factory) = gst::ElementFactory::find(name) {
        let caps = get_src_caps(&factory, pad);

        Some(BashGstElement {
            factory: factory,
            element: gst::ElementFactory::make(name, None).unwrap(),
            caps: caps,
        })
    } else {
        None
    }
}

pub fn init() {
    gst::init().unwrap();
}

mod tests {
    use super::*;

    #[test]
    fn test1() {
        assert_eq!(
            gst::ElementFactory::list_filter(
                &LIST,
                &gst::Caps::new_empty(),
                gst::PadDirection::Sink,
                false,
            ),
            Vec::<ElementFactory>::new()
        )
    }

    #[test]
    fn test2() {
        gst::init().unwrap();

        let caps = gst::Caps::new_any();
        let factory = gst::ElementFactory::find("oggmux").unwrap();

        assert_ne!(factory.can_sink_all_caps(&caps), true);
    }

    #[test]
    fn test3() {
        gst::init().unwrap();

        let caps = gst::Caps::new_any();
        let factory = gst::ElementFactory::find("filesink").unwrap();

        assert!(factory.can_sink_all_caps(&caps));
    }

    #[test]
    fn test4() {
        gst::init().unwrap();
        let element = find_element("udpsink", None).unwrap();

        for x in element.get_property_names(&[], None) {
            println!("-- {:?}", x);
        }
    }
}
