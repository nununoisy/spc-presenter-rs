mod formatter;
mod functions;

use std::str::FromStr;
use fluent::{FluentArgs, FluentBundle, FluentMessage, FluentResource};
use unic_langid::{langid, LanguageIdentifier};
use formatter::fluent_formatter;
use functions::fluent_fn_number;

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
                make_bundle!("en-US"),
                make_bundle!("en-CA"),
                make_bundle!("en-GB", "en-NZ", "en-AU", "en-IE"),
                make_bundle!("es-ES")
            ],
            language_id
        }
    }

    fn bundle(&self, message_id: Option<&str>) -> Option<&FluentBundle<FluentResource>> {
        // First check for an exact match
        for bundle in self.bundles.iter() {
            if let Some(message_id) = message_id {
                if !bundle.has_message(message_id) {
                    continue;
                }
            }
            if bundle.locales.iter().any(|locale| &self.language_id == locale) {
                return Some(bundle);
            }
        }

        // Now match the language
        for bundle in self.bundles.iter() {
            if let Some(message_id) = message_id {
                if !bundle.has_message(message_id) {
                    continue;
                }
            }
            if bundle.locales.iter().any(|locale| self.language_id.language == locale.language) {
                return Some(bundle);
            }
        }

        // No match :(
        None
    }

    pub fn languages(&self) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();

        for bundle in self.bundles.iter() {
            result.push(bundle.locales[0].to_string());
        }

        result
    }

    pub fn language(&self) -> Option<String> {
        self.bundle(None)
            .map(|bundle| bundle.locales[0].to_string())
    }

    pub fn set_language(&mut self, language_id: &str) {
        if let Ok(language_id) = language_id.parse::<LanguageIdentifier>() {
            self.language_id = language_id;
        }
    }

    pub fn get(&self, message_id: &str, args: Option<&FluentArgs>, strip_fsi_pdi: bool) -> String {
        if let Some(bundle) = self.bundle(Some(message_id)) {
            let message = bundle.get_message(message_id).unwrap();
            if let Some(pattern) = message.value() {
                let mut errors = Vec::new();
                let result = bundle.format_pattern(&pattern, args, &mut errors);
                return if strip_fsi_pdi {
                    result.replace(&['\u{2068}', '\u{2069}'], "")
                } else {
                    result.to_string()
                }
            }
        }
        message_id.to_string()
    }
}

macro_rules! fluent_args {
    ($k: tt : $v: expr $(, $mk: tt : $mv: expr)*) => {{
        let mut args = FluentArgs::new();

        args.set(stringify!($k), $v);
        $( args.set(stringify!($mk), $mv); )*

        args
    }};
}
pub(crate) use fluent_args;
