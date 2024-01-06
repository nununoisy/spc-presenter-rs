use fluent::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use fluent::memoizer::MemoizerKind;
use fluent::types::FluentNumberStyle;
use unic_langid::{langid, LanguageIdentifier};

macro_rules! fluent_fn_arg {
    ($args: ident [ $key: expr ]) => {{
        match $args.get($key).cloned() {
            Some(v) => v,
            None => return FluentValue::Error
        }
    }};
    ($args: ident [ $key: expr ] -> $t: tt) => {{
        match $args.get($key).cloned() {
            Some(FluentValue::$t(v)) => v,
            _ => return FluentValue::Error
        }
    }};
}

fn fluent_formatter<M: MemoizerKind>(value: &FluentValue<'_>, _memoizer: &M) -> Option<String> {
    match value {
        FluentValue::Number(num) => {
            match num.options.style {
                FluentNumberStyle::Decimal => {
                    let i_min = num.options.minimum_integer_digits.unwrap_or(0);
                    let f_min = num.options.minimum_fraction_digits.unwrap_or(0);
                    let f_max = num.options.maximum_fraction_digits.unwrap_or(64);

                    if num.value.fract() == 0.0 && f_min == 0 {
                        return Some(format!("{:0>i_min$}", num.value));
                    }

                    let num_str = format!("{:?}", num.value);
                    let (i_part, mut f_part) = num_str
                        .split_once('.')
                        .map(|(i, f)| (i.to_string(), f.to_string()))
                        .unwrap();

                    if f_part.len() > f_max {
                        f_part.truncate(f_max + 1);
                        f_part = (f_part.parse::<f64>().unwrap() / 10.0).round().to_string();
                    }
                    Some(format!("{:0>i_min$}.{:0<f_min$}", i_part, f_part))
                }
                FluentNumberStyle::Percent => Some(format!("{}%", (num.value * 100.0).round())),
                FluentNumberStyle::Currency => None
            }
        },
        _ => None
    }
}

fn fluent_fn_number<'a>(args: &[FluentValue<'a>], named_args: &FluentArgs<'_>) -> FluentValue<'a> {
    let num_val = match fluent_fn_arg!(args[0]) {
        FluentValue::String(s) => FluentValue::try_number(s),
        other => other
    };
    let mut num = match num_val {
        FluentValue::Number(n) => n,
        other => return other
    };

    num.options.merge(named_args);

    FluentValue::Number(num)
}

pub struct LocalizationAdapter {
    bundles: Vec<FluentBundle<FluentResource>>,
    language_id: LanguageIdentifier
}

macro_rules! make_bundle {
    ($base: literal $(, $extras: literal)*) => {{
        let ftl_string = include_str!(concat!("../slint/localization/", $base, ".ftl")).to_string();
        let resource = FluentResource::try_new(ftl_string).expect(concat!("Failed to parse FTL for locale ", $base));

        let mut bundle = FluentBundle::new(vec![langid!($base) $(, langid!($extras))*]);
        bundle.add_resource(resource).expect(concat!("Failed to add FTL resource to bundle for locale ", $base));
        bundle.set_formatter(Some(fluent_formatter));
        bundle.add_function("NUMBER", fluent_fn_number).expect(concat!("Failed to add NUMBER function to bundle for locale ", $base));

        bundle
    }};
}

impl LocalizationAdapter {
    pub fn new() -> Self {
        let language_id = sys_locale::get_locale()
            .and_then(|locale| locale.parse::<LanguageIdentifier>().ok())
            .unwrap_or(langid!("en-US"));

        Self {
            bundles: vec![
                make_bundle!("en", "en-US"),
                make_bundle!("en-GB", "en-NZ", "en-AU"),
                make_bundle!("es", "es-ES")
            ],
            language_id
        }
    }

    pub fn set_language(&mut self, language_id: &str) {
        if let Ok(language_id) = language_id.parse::<LanguageIdentifier>() {
            self.language_id = language_id;
        }
    }

    pub fn bundle(&self) -> &FluentBundle<FluentResource> {
        // First check for an exact match
        for bundle in self.bundles.iter() {
            if bundle.locales.iter().any(|locale| &self.language_id == locale) {
                return bundle;
            }
        }
        // Now match the language
        for bundle in self.bundles.iter() {
            if bundle.locales.iter().any(|locale| self.language_id.language == locale.language) {
                return bundle;
            }
        }
        // Fallback
        self.bundles.get(0).unwrap()
    }

    pub fn get(&self, message_id: &str, args: Option<&FluentArgs>) -> String {
        let bundle = self.bundle();

        if let Some(message) = bundle.get_message(message_id) {
            if let Some(pattern) = message.value() {
                let mut errors = Vec::new();
                return bundle.format_pattern(&pattern, args, &mut errors).to_string();
            }
        }
        message_id.to_string()
    }
}

macro_rules! fluent_args {
    ($k: tt : $v: expr $(, $mk: literal : $mv: expr)*) => {{
        let mut args = FluentArgs::new();

        args.set(stringify!($k), $v);
        $( args.set(stringify!($mk), $mv); )*

        args
    }};
}
pub(crate) use fluent_args;
