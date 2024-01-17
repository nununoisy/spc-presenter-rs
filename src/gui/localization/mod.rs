mod formatter;
mod functions;

use fluent::{FluentArgs, FluentBundle, FluentResource};
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
                make_bundle!("en", "en-US"),
                make_bundle!("en-CA"),
                make_bundle!("en-GB", "en-NZ", "en-AU", "en-IE"),
                make_bundle!("es")
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

    pub fn get(&self, message_id: &str, args: Option<&FluentArgs>, strip_fsi_pdi: bool) -> String {
        let bundle = self.bundle();

        if let Some(message) = bundle.get_message(message_id) {
            if let Some(pattern) = message.value() {
                let mut errors = Vec::new();
                let result = bundle.format_pattern(&pattern, args, &mut errors);
                if strip_fsi_pdi {
                    return result.replace(&['\u{2068}', '\u{2069}'], "");
                } else {
                    return result.to_string();
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
