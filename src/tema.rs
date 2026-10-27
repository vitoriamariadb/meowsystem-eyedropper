// SPDX-License-Identifier: MPL-2.0

//! A extensão de tema: um arquivo que o theme pack do MeowSystem pode trazer
//! com as cores deste applet. Sem o arquivo, vale o tema do COSMIC inteiro;
//! uma linha que não se entende é pulada, nunca derruba o applet.
//!
//! `$MEOWSYSTEM_TEMA`, ou `$XDG_CONFIG_HOME/meowsystem/extensoes/<APP_ID>/tema.conf`:
//!
//! ```text
//! # cores só em #rrggbb ou #rrggbbaa; quem traduz o nome da paleta é o MeowSystem
//! acento = #cba6f7
//! fundo = #1e1e2e
//! ```

use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use cosmic::cosmic_theme::{
    Component, Container, Theme as CosmicTheme, ThemeBuilder,
    palette::{Srgb, Srgba},
};

/// Uma cor do arquivo, em bytes: `#rrggbb` tem alfa 255.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cor(pub [u8; 4]);

impl Cor {
    pub fn parse(text: &str) -> Option<Cor> {
        let hex = text.strip_prefix('#')?;
        if !(hex.len() == 6 || hex.len() == 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
        Some(Cor([byte(0)?, byte(2)?, byte(4)?, alpha]))
    }

    pub fn srgba(self) -> Srgba {
        let [r, g, b, a] = self.0.map(|c| f32::from(c) / 255.0);
        Srgba::new(r, g, b, a)
    }

    pub fn srgb(self) -> Srgb {
        let [r, g, b, _] = self.0.map(|c| f32::from(c) / 255.0);
        Srgb::new(r, g, b)
    }

    pub fn iced(self) -> cosmic::iced::Color {
        let [r, g, b, a] = self.0;
        cosmic::iced::Color::from_rgba8(r, g, b, f32::from(a) / 255.0)
    }
}

/// As chaves de cor que o arquivo aceita, e o que cada uma pinta.
pub const CORES: [(&str, &str); 7] = [
    ("acento", "botões principais, escolhas marcadas e interruptores ligados"),
    ("destaque", "texto e ícones sobre o acento"),
    ("fundo", "fundo do popup"),
    ("borda", "contorno e divisórias do popup"),
    ("texto", "texto e ícones do popup"),
    ("bloco", "fundo dos botões, dos campos e das linhas do popup"),
    ("tinta", "o ícone do applet na barra do painel"),
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tema {
    pub cores: BTreeMap<String, Cor>,
    pub origem: Option<PathBuf>,
    pub ignoradas: usize,
}

impl Tema {
    pub fn cor(&self, chave: &str) -> Option<Cor> {
        self.cores.get(chave).copied()
    }

    #[cfg(test)]
    pub fn vazio(&self) -> bool {
        self.cores.is_empty()
    }

    /// Lê o texto do arquivo; o que não se entende fica de fora, avisado.
    pub fn parse(text: &str) -> Tema {
        let mut tema = Tema::default();
        for (numero, linha) in text.lines().enumerate() {
            let linha = linha.trim();
            if linha.is_empty() || linha.starts_with('#') {
                continue;
            }
            let Some((chave, valor)) = linha.split_once('=') else {
                log::warn!("tema, linha {}: sem «=»", numero + 1);
                tema.ignoradas += 1;
                continue;
            };
            let (chave, valor) = (chave.trim(), valor.trim());
            if CORES.iter().any(|(nome, _)| *nome == chave)
                && let Some(cor) = Cor::parse(valor)
            {
                tema.cores.insert(chave.to_owned(), cor);
                continue;
            }
            log::warn!("tema, linha {}: «{chave}» com «{valor}» não vale", numero + 1);
            tema.ignoradas += 1;
        }
        tema
    }

    /// O arquivo de agora; sem ele, o tema vazio, que é o do COSMIC.
    pub fn load(app_id: &str) -> Tema {
        let Some(path) = caminho(app_id) else {
            return Tema::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => Tema {
                origem: Some(path),
                ..Tema::parse(&text)
            },
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("cannot read {}: {err}", path.display());
                }
                Tema::default()
            }
        }
    }

    /// O tema do COSMIC com as cores do arquivo por cima, ou nada quando o
    /// arquivo não traz cor para o popup (vazio, ou só `tinta`).
    pub fn cosmic(&self, dark: bool) -> Option<cosmic::Theme> {
        let popup = ["acento", "destaque", "fundo", "borda", "texto", "bloco"];
        if !popup.iter().any(|chave| self.cores.contains_key(*chave)) {
            return None;
        }
        let mut builder = if dark {
            ThemeBuilder::dark_config()
                .ok()
                .and_then(|h| cosmic::cosmic_config::CosmicConfigEntry::get_entry(&h).ok())
                .unwrap_or_else(ThemeBuilder::dark)
        } else {
            ThemeBuilder::light_config()
                .ok()
                .and_then(|h| cosmic::cosmic_config::CosmicConfigEntry::get_entry(&h).ok())
                .unwrap_or_else(ThemeBuilder::light)
        };
        if let Some(cor) = self.cor("acento") {
            builder = builder.accent(cor.srgb());
        }
        if let Some(cor) = self.cor("fundo") {
            builder = builder.bg_color(cor.srgba());
        }
        let mut theme = builder.build();
        self.cravar(&mut theme);
        Some(cosmic::Theme::custom(Arc::new(theme)))
    }

    /// As cores exatas do arquivo, por cima do que o construtor derivou.
    pub fn cravar(&self, theme: &mut CosmicTheme) {
        if let Some(cor) = self.cor("acento") {
            theme.accent.base = cor.srgba();
            theme.accent_button.base = cor.srgba();
        }
        if let Some(cor) = self.cor("destaque") {
            theme.accent.on = cor.srgba();
            theme.accent_button.on = cor.srgba();
        }
        let mut fundos = [theme.background(false).clone(), theme.background(true).clone()];
        for fundo in &mut fundos {
            self.pintar_fundo(fundo);
        }
        if (fundos[0] != *theme.background(false) || fundos[1] != *theme.background(true))
            && let Some(novo) = trocar_fundos(theme, &fundos)
        {
            *theme = novo;
        }
    }

    fn pintar_fundo(&self, fundo: &mut Container) {
        if let Some(cor) = self.cor("fundo") {
            fundo.base = cor.srgba();
        }
        if let Some(cor) = self.cor("borda") {
            fundo.divider = cor.srgba();
            fundo.component.divider = cor.srgba();
        }
        if let Some(cor) = self.cor("texto") {
            fundo.on = cor.srgba();
            fundo.component.on = cor.srgba();
        }
        if let Some(cor) = self.cor("bloco") {
            pintar(&mut fundo.component, cor);
        }
    }
}

fn trocar_fundos(theme: &CosmicTheme, fundos: &[Container; 2]) -> Option<CosmicTheme> {
    let troca = || -> serde_json::Result<CosmicTheme> {
        let mut value = serde_json::to_value(theme)?;
        value["background"] = serde_json::to_value(&fundos[0])?;
        value["transparent_background"] = serde_json::to_value(&fundos[1])?;
        serde_json::from_value(value)
    };
    troca()
        .inspect_err(|err| log::warn!("cannot apply the theme's background: {err}"))
        .ok()
}

/// Um componente com fundo novo: hover e pressionado puxam para a cor do texto.
fn pintar(component: &mut Component, cor: Cor) {
    let on = component.on;
    let mix = |t: f32| {
        let c = cor.srgba();
        Srgba::new(
            c.red + (on.red - c.red) * t,
            c.green + (on.green - c.green) * t,
            c.blue + (on.blue - c.blue) * t,
            c.alpha,
        )
    };
    component.base = cor.srgba();
    component.hover = mix(0.1);
    component.pressed = mix(0.2);
}

/// O botão do applet na barra: o ícone na tinta do pack, ou, sem ela, o do
/// próprio painel.
pub fn botao<'a, Message: Clone + 'static>(
    applet: &cosmic::applet::Context,
    nome: &'a str,
    tinta: Option<Cor>,
) -> cosmic::widget::Button<'a, Message> {
    let Some(tinta) = tinta else {
        return applet.icon_button(nome);
    };
    let lado = applet.suggested_size(true).0;
    let icone = cosmic::widget::icon(
        cosmic::widget::icon::from_name(nome)
            .symbolic(true)
            .size(lado)
            .into(),
    )
    .class(cosmic::theme::Svg::Custom(std::rc::Rc::new(move |_| {
        cosmic::iced::widget::svg::Style {
            color: Some(tinta.iced()),
        }
    })))
    .width(cosmic::iced::Length::Fixed(f32::from(lado)))
    .height(cosmic::iced::Length::Fixed(f32::from(lado)));
    applet.button_from_element(icone, true)
}

pub fn caminho(app_id: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("MEOWSYSTEM_TEMA").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(sob(&base, app_id))
}

fn sob(base: &std::path::Path, app_id: &str) -> PathBuf {
    base.join("meowsystem").join("extensoes").join(app_id).join("tema.conf")
}

#[cfg(test)]
mod tests {
    use super::*;

    static AMBIENTE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn cor_aceita_so_hex() {
        assert_eq!(Cor::parse("#cba6f7"), Some(Cor([0xcb, 0xa6, 0xf7, 255])));
        assert_eq!(Cor::parse("#1e1e2ecc"), Some(Cor([0x1e, 0x1e, 0x2e, 0xcc])));
        for ruim in ["cba6f7", "#cba6f", "#gggggg", "mauve", "#cba6f7a", "", "#ção12"] {
            assert_eq!(Cor::parse(ruim), None, "{ruim}");
        }
    }

    #[test]
    fn arquivo_completo() {
        let tema = Tema::parse(
            "# Catppuccin Mocha, mauve\n\
             acento = #cba6f7\n\
             \n\
             fundo=#1e1e2e\n\
             tinta = #cdd6f4  \n",
        );
        assert_eq!(tema.cor("acento"), Some(Cor([0xcb, 0xa6, 0xf7, 255])));
        assert_eq!(tema.cor("fundo"), Some(Cor([0x1e, 0x1e, 0x2e, 255])));
        assert_eq!(tema.cor("tinta"), Some(Cor([0xcd, 0xd6, 0xf4, 255])));
        assert_eq!(tema.ignoradas, 0);
    }

    #[test]
    fn linha_ruim_e_pulada_sem_derrubar() {
        let tema = Tema::parse(
            "acento = mauve\n\
             cor-que-nao-existe = #ffffff\n\
             sem igual\n\
             icone.qualquer = foo\n\
             texto = #ffffff\n",
        );
        assert_eq!(tema.ignoradas, 4);
        assert_eq!(tema.cores.len(), 1);
    }

    #[test]
    fn vazio_e_lixo_dao_o_tema_do_cosmic() {
        assert!(Tema::parse("").vazio());
        assert!(Tema::parse("\u{0}\u{ff}###\n=\n==\n").vazio());
        assert!(Tema::parse("tinta = #ffffff").cosmic(false).is_none());
    }

    #[test]
    fn arquivo_ausente_nao_derruba() {
        let _trava = AMBIENTE.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: os testes deste módulo são os únicos a mexer na variável.
        unsafe { std::env::set_var("MEOWSYSTEM_TEMA", "/nao/existe/tema.conf") };
        let tema = Tema::load("qualquer");
        unsafe { std::env::remove_var("MEOWSYSTEM_TEMA") };
        assert!(tema.vazio());
        assert_eq!(tema.origem, None);
    }

    #[test]
    fn cores_cravadas_sao_exatas() {
        let mut theme = ThemeBuilder::dark().build();
        let tema = Tema::parse("acento = #cba6f7\nfundo = #1e1e2e\ntexto = #cdd6f4\nbloco = #313244\n");
        tema.cravar(&mut theme);
        let hex = |c: Srgba| {
            let c: cosmic::cosmic_theme::palette::rgb::Rgba<cosmic::cosmic_theme::palette::encoding::Srgb, u8> =
                c.into_format();
            [c.red, c.green, c.blue]
        };
        assert_eq!(hex(theme.accent_color()), [0xcb, 0xa6, 0xf7]);
        for transparent in [false, true] {
            let fundo = theme.background(transparent);
            assert_eq!(hex(fundo.base), [0x1e, 0x1e, 0x2e]);
            assert_eq!(hex(fundo.on), [0xcd, 0xd6, 0xf4]);
            assert_eq!(hex(fundo.component.base), [0x31, 0x32, 0x44]);
            assert_ne!(fundo.component.hover, fundo.component.base);
        }
    }

    #[test]
    fn caminho_usa_o_app_id() {
        assert_eq!(
            sob(std::path::Path::new("/tmp/cfg-teste"), "io.github.meowsystem.Exemplo"),
            PathBuf::from("/tmp/cfg-teste/meowsystem/extensoes/io.github.meowsystem.Exemplo/tema.conf")
        );
    }

    #[test]
    fn le_o_tema_conf_que_o_meowsystem_escreve_no_home() {
        let _trava = AMBIENTE.lock().unwrap_or_else(|e| e.into_inner());
        let app_id = <crate::app::AppModel as cosmic::Application>::APP_ID;
        let casa = std::env::temp_dir().join(format!("meowsystem-eyedropper-casa-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&casa);
        let arquivo = casa
            .join(".config/meowsystem/extensoes")
            .join(app_id)
            .join("tema.conf");
        std::fs::create_dir_all(arquivo.parent().unwrap()).unwrap();
        std::fs::write(
            &arquivo,
            "acento = #cba6f7\ndestaque = #1e1e2e\nfundo = #1e1e2e\nborda = #45475a\n\
             texto = #cdd6f4\nbloco = #313244\ntinta = #cdd6f4",
        )
        .unwrap();
        let antes: Vec<_> = ["HOME", "XDG_CONFIG_HOME", "MEOWSYSTEM_TEMA"]
            .iter()
            .map(|nome| (*nome, std::env::var_os(nome)))
            .collect();
        // SAFETY: os testes deste módulo são os únicos a mexer nas variáveis, sob a trava.
        unsafe {
            std::env::set_var("HOME", &casa);
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::remove_var("MEOWSYSTEM_TEMA");
        }
        let tema = Tema::load(app_id);
        let popup = tema.cosmic(true);
        for (nome, valor) in antes {
            // SAFETY: idem.
            unsafe {
                match valor {
                    Some(v) => std::env::set_var(nome, v),
                    None => std::env::remove_var(nome),
                }
            }
        }
        let _ = std::fs::remove_dir_all(&casa);

        assert_eq!(tema.origem, Some(arquivo));
        assert_eq!(tema.ignoradas, 0);
        assert_eq!(tema.cores.len(), CORES.len());
        assert_eq!(tema.cor("acento"), Cor::parse("#cba6f7"));
        assert_eq!(tema.cor("tinta"), Cor::parse("#cdd6f4"));
        assert!(popup.is_some());
    }
}
