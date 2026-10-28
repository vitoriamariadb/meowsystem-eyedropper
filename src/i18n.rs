// SPDX-License-Identifier: MPL-2.0

//! Provides localization support for this crate.

use std::sync::LazyLock;

use i18n_embed::{
    DefaultLocalizer, LanguageLoader, Localizer,
    fluent::{FluentLanguageLoader, fluent_language_loader},
    unic_langid::LanguageIdentifier,
};
use rust_embed::RustEmbed;

/// Applies the requested language(s) to requested translations from the `fl!()` macro.
pub fn init(requested_languages: &[LanguageIdentifier]) {
    if let Err(why) = localizer().select(requested_languages) {
        log::error!("error while loading fluent localizations: {why}");
    }
}

// Get the `Localizer` to be used for localizing this library.
#[must_use]
pub fn localizer() -> Box<dyn Localizer> {
    Box::from(DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations))
}

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();

    loader
        .load_fallback_language(&Localizations)
        .expect("Error while loading fallback language");

    loader
});

/// Request a localized string by ID from the i18n/ directory.
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

#[cfg(test)]
mod tests {
    fn chaves(ftl: &str) -> Vec<&str> {
        ftl.lines()
            .filter_map(|linha| linha.split_once('=').map(|(chave, _)| chave.trim()))
            .filter(|chave| !chave.is_empty() && !chave.starts_with('#'))
            .collect()
    }

    #[test]
    fn pt_br_traz_toda_chave_do_ingles() {
        let en = chaves(include_str!("../i18n/en/meowsystem_eyedropper.ftl"));
        let pt = chaves(include_str!("../i18n/pt-BR/meowsystem_eyedropper.ftl"));
        let faltam: Vec<_> = en.iter().filter(|chave| !pt.contains(chave)).collect();
        assert!(faltam.is_empty(), "faltam em pt-BR: {faltam:?}");
    }

    #[test]
    fn sobe_em_pt_br_com_lang_pt_br() {
        unsafe {
            std::env::set_var("LANG", "pt_BR.UTF-8");
            for nome in ["LANGUAGE", "LC_ALL", "LC_MESSAGES"] {
                std::env::remove_var(nome);
            }
        }
        super::init(&i18n_embed::DesktopLanguageRequester::requested_languages());
        assert_eq!(crate::fl!("select-colour"), "Escolher cor");
        assert_eq!(crate::fl!("no-color-selected"), "Nenhuma cor escolhida");
        assert_eq!(crate::fl!("copy-on-select"), "Copiar ao escolher");
    }
}
