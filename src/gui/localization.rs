use fluent::{FluentBundle, FluentResource};
use unic_langid::{langid, LanguageIdentifier};

pub struct LocalizationAdapter {
    bundles: Vec<FluentBundle<FluentResource>>,
    language_id: LanguageIdentifier
}

macro_rules! make_bundle {
    ($base: literal $(, $extras: literal)*) => {{
        let ftl_string = include_str!(concat!("slint/localization/", $base, ".ftl")).to_string();
        let resource = FluentResource::try_new(ftl_string).expect(concat!("Failed to parse FTL for locale ", $base));

        let mut bundle = FluentBundle::new(vec![langid!($base) $(, langid!($extras))*]);
        bundle.add_resource(resource).expect(concat!("Failed to add FTL resource to bundle for locale ", $base));

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
                make_bundle!("en-US", "en-GB"),
                make_bundle!("es-ES", "es-MX")
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
        for bundle in self.bundles.iter() {
            if bundle.locales.iter().any(|locale| self.language_id.language == locale.language) {
                return bundle;
            }
        }
        self.bundles.get(0).unwrap()
    }
}
