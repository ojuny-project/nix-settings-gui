// Hardware profile detection mirrors the coverage table in ojuny-template's
// modules/backend/10-hardware-profile.nix and scripts/regenerate-hardware.py.
// It is kept here so the widget can describe a facter report without running
// Nix or nix eval.
pub(crate) struct ProfileRow {
    pub(crate) vendor: &'static str,
    pub(crate) pattern: &'static str,
    pub(crate) profile: &'static str,
    pub(crate) state: &'static str,
}

pub(crate) const HARDWARE_PROFILES: &[ProfileRow] = &[
    ProfileRow {
        vendor: "apple",
        pattern: ".*macbookpro.*",
        profile: "apple-macbook-pro",
        state: "untested",
    },
    ProfileRow {
        vendor: "apple",
        pattern: ".*macbookair7,.*",
        profile: "apple-macbook-air-3",
        state: "untested",
    },
    ProfileRow {
        vendor: "apple",
        pattern: ".*macbookair8,.*",
        profile: "apple-macbook-air-4",
        state: "untested",
    },
    ProfileRow {
        vendor: "apple",
        pattern: ".*macbookair9,.*",
        profile: "apple-macbook-air-5",
        state: "untested",
    },
    ProfileRow {
        vendor: "apple",
        pattern: ".*macmini.*",
        profile: "apple-macmini-4-1",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*g533q.*",
        profile: "asus-rog-strix-g533q",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*g533zw.*",
        profile: "asus-rog-strix-g533zw",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*ga401iv.*",
        profile: "asus-zephyrus-ga401iv",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*ga401.*",
        profile: "asus-zephyrus-ga401",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*ga402x.*",
        profile: "asus-zephyrus-ga402x-amdgpu",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*ga402.*",
        profile: "asus-zephyrus-ga402",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*hn7306eac.*",
        profile: "asus-proart-px13-hn7306eac",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*px13.*",
        profile: "asus-proart-px13",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*g513.*",
        profile: "asus-rog-strix-g513im",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*g713.*",
        profile: "asus-rog-strix-g713ie",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*g733.*",
        profile: "asus-rog-strix-g733qs",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*ga503.*",
        profile: "asus-zephyrus-ga503",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*gu603.*",
        profile: "asus-zephyrus-gu603h",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*ux371.*",
        profile: "asus-zenbook-ux371",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*ux535.*",
        profile: "asus-zenbook-ux535",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*fx506.*",
        profile: "asus-fx506hm",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*fa506.*",
        profile: "asus-fa506nc",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*fa507.*",
        profile: "asus-fa507nv",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*rog strix x570.*",
        profile: "asus-rog-strix-x570e",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*rog ally rc71l.*",
        profile: "asus-ally-rc71l",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*gv302x.*",
        profile: "asus-flow-gv302x-amdgpu",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*gz301vu.*",
        profile: "asus-flow-gz301vu",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*gl552vw.*",
        profile: "asus-rog-gl552vw",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*ga502.*",
        profile: "asus-zephyrus-ga502",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*gu605cw.*",
        profile: "asus-zephyrus-gu605cw",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*gu605my.*",
        profile: "asus-zephyrus-gu605my",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*pro ws x570.*",
        profile: "asus-pro-ws-x570-ace",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*b9406.*",
        profile: "ojuny-asus-expertbook-b9406",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*ux5406aa.*",
        profile: "ojuny-asus-zenbook-ux5406aa",
        state: "untested",
    },
    ProfileRow {
        vendor: "asus",
        pattern: ".*gz302.*",
        profile: "ojuny-asus-rog-z13-gz302",
        state: "untested",
    },
    ProfileRow {
        vendor: "chuwi",
        pattern: ".*minibook x.*",
        profile: "chuwi-minibook-x",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 9370.*",
        profile: "dell-xps-13-9370",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 9380.*",
        profile: "dell-xps-13-9380",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 9300.*",
        profile: "dell-xps-13-9300",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 9310.*",
        profile: "dell-xps-13-9310",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 9315.*",
        profile: "dell-xps-13-9315",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 9343.*",
        profile: "dell-xps-13-9343",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 15 9500.*",
        profile: "dell-xps-15-9500",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 15 9510.*",
        profile: "dell-xps-15-9510",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 15 9520.*",
        profile: "dell-xps-15-9520",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 15 9530.*",
        profile: "dell-xps-15-9530",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 15 9550.*",
        profile: "dell-xps-15-9550",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*latitude 7420.*",
        profile: "dell-latitude-7420",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*latitude 7430.*",
        profile: "dell-latitude-7430",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*latitude 7490.*",
        profile: "dell-latitude-7490",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*precision 5560.*",
        profile: "dell-precision-5560",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*precision 5570.*",
        profile: "dell-precision-5570",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 7390.*",
        profile: "dell-xps-13-7390",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 9333.*",
        profile: "dell-xps-13-9333",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 9350.*",
        profile: "dell-xps-13-9350",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 13 9360.*",
        profile: "dell-xps-13-9360",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 15 7590.*",
        profile: "dell-xps-15-7590",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 15 9560.*",
        profile: "dell-xps-15-9560-nvidia",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 15 9570.*",
        profile: "dell-xps-15-9570-nvidia",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 17 9700.*",
        profile: "dell-xps-17-9700-intel",
        state: "untested",
    },
    ProfileRow {
        vendor: "dell",
        pattern: ".*xps 17 9710.*",
        profile: "dell-xps-17-9710-intel",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 13.*7040.*",
        profile: "framework-13-7040-amd",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 13.*12th.*",
        profile: "framework-12th-gen-intel",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 13.*11th.*",
        profile: "framework-11th-gen-intel",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 13.*13th.*",
        profile: "framework-13th-gen-intel",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 13.*core ultra.*1.*",
        profile: "framework-intel-core-ultra-series1",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 13.*ai 300.*",
        profile: "framework-amd-ai-300-series",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 16.*7040.*",
        profile: "framework-16-7040-amd",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 16.*ai 300.*",
        profile: "framework-16-amd-ai-300-series",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 12.*13th.*",
        profile: "framework-12-13th-gen-intel",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*laptop 13.*core ultra.*3.*",
        profile: "framework-intel-core-ultra-series3",
        state: "untested",
    },
    ProfileRow {
        vendor: "framework",
        pattern: ".*framework desktop.*",
        profile: "framework-desktop-amd-ai-max-300-series",
        state: "untested",
    },
    ProfileRow {
        vendor: "gigabyte",
        pattern: ".*b550.*",
        profile: "gigabyte-b550",
        state: "untested",
    },
    ProfileRow {
        vendor: "gigabyte",
        pattern: ".*b650.*",
        profile: "gigabyte-b650",
        state: "untested",
    },
    ProfileRow {
        vendor: "google",
        pattern: ".*pixelbook.*",
        profile: "google-pixelbook",
        state: "untested",
    },
    ProfileRow {
        vendor: "google",
        pattern: ".*brya.*",
        profile: "google-brya",
        state: "untested",
    },
    ProfileRow {
        vendor: "google",
        pattern: ".*rex.*",
        profile: "google-rex",
        state: "untested",
    },
    ProfileRow {
        vendor: "gpd",
        pattern: ".*pocket 3.*",
        profile: "gpd-pocket-3",
        state: "untested",
    },
    ProfileRow {
        vendor: "gpd",
        pattern: ".*pocket 4.*",
        profile: "gpd-pocket-4",
        state: "untested",
    },
    ProfileRow {
        vendor: "gpd",
        pattern: ".*win 2.*",
        profile: "gpd-win-2",
        state: "untested",
    },
    ProfileRow {
        vendor: "gpd",
        pattern: ".*win max 2.*",
        profile: "gpd-win-max-2-2023",
        state: "untested",
    },
    ProfileRow {
        vendor: "gpd",
        pattern: ".*win mini.*",
        profile: "gpd-win-mini-2024",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*elitebook 830 g6.*",
        profile: "hp-elitebook-830g6",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*elitebook 845 g7.*",
        profile: "hp-elitebook-845g7",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*elitebook 845 g8.*",
        profile: "hp-elitebook-845g8",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*elitebook 845 g9.*",
        profile: "hp-elitebook-845g9",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*probook 440 g5.*",
        profile: "hp-probook-440G5",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*probook 460 g11.*",
        profile: "hp-probook-460G11",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*laptop 14s.*",
        profile: "hp-laptop-14s-dq2024nf",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*laptop 15s.*",
        profile: "hp-laptop-15s-fq1xxx",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*omen 14.*",
        profile: "omen-14-fb0798ng",
        state: "untested",
    },
    ProfileRow {
        vendor: "hp",
        pattern: ".*omen 16.*",
        profile: "omen-16-n0005ne",
        state: "untested",
    },
    ProfileRow {
        vendor: "huawei",
        pattern: ".*machc.*",
        profile: "huawei-machc-wa",
        state: "untested",
    },
    ProfileRow {
        vendor: "intel",
        pattern: ".*nuc.*",
        profile: "intel-nuc-12wshi7",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*x1 carbon.*7th.*",
        profile: "lenovo-thinkpad-x1-7th-gen",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*x1 carbon.*9th.*",
        profile: "lenovo-thinkpad-x1-9th-gen",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*x1 carbon.*10th.*",
        profile: "lenovo-thinkpad-x1-10th-gen",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*x1 carbon.*11th.*",
        profile: "lenovo-thinkpad-x1-11th-gen",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*20qd.*",
        profile: "lenovo-thinkpad-x1-7th-gen",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*20xw.*",
        profile: "lenovo-thinkpad-x1-9th-gen",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*21cb.*",
        profile: "lenovo-thinkpad-x1-10th-gen",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*21hm.*",
        profile: "lenovo-thinkpad-x1-11th-gen",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*thinkpad x1 carbon.*",
        profile: "lenovo-thinkpad-x1",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*thinkpad t14.*gen 1.*",
        profile: "lenovo-thinkpad-t14-intel-gen1",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*thinkpad t14s.*",
        profile: "lenovo-thinkpad-t14s",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*thinkpad x13.*",
        profile: "lenovo-thinkpad-x13-intel",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*yoga 7 14arh7.*",
        profile: "lenovo-yoga-7-14ARH7-amdgpu",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*yoga 6 13alc6.*",
        profile: "lenovo-yoga-6-13ALC6",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*ideapad 15ach6.*",
        profile: "lenovo-ideapad-15ach6",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*legion 5.*",
        profile: "lenovo-legion-15ach6h",
        state: "untested",
    },
    ProfileRow {
        vendor: "lenovo",
        pattern: ".*yoga pro 7 14iah10.*",
        profile: "ojuny-lenovo-yoga-pro-7-14iah10",
        state: "untested",
    },
    ProfileRow {
        vendor: "microsoft",
        pattern: ".*surface go.*",
        profile: "microsoft-surface-go",
        state: "untested",
    },
    ProfileRow {
        vendor: "microsoft",
        pattern: ".*surface pro 3.*",
        profile: "microsoft-surface-pro-3",
        state: "untested",
    },
    ProfileRow {
        vendor: "microsoft",
        pattern: ".*surface pro 9.*",
        profile: "microsoft-surface-pro-9",
        state: "untested",
    },
    ProfileRow {
        vendor: "microsoft",
        pattern: ".*surface pro.*",
        profile: "microsoft-surface-pro-intel",
        state: "untested",
    },
    ProfileRow {
        vendor: "microsoft",
        pattern: ".*surface laptop.*amd.*",
        profile: "microsoft-surface-laptop-amd",
        state: "untested",
    },
    ProfileRow {
        vendor: "msi",
        pattern: ".*gf63.*",
        profile: "msi-gf63",
        state: "untested",
    },
    ProfileRow {
        vendor: "msi",
        pattern: ".*gs60.*",
        profile: "msi-gs60",
        state: "untested",
    },
    ProfileRow {
        vendor: "msi",
        pattern: ".*gl62.*",
        profile: "msi-gl62",
        state: "untested",
    },
    ProfileRow {
        vendor: "msi",
        pattern: ".*gl65.*",
        profile: "msi-gl65-10SDR-492",
        state: "untested",
    },
    ProfileRow {
        vendor: "msi",
        pattern: ".*prestige 15.*",
        profile: "msi-prestige-15-a10sc",
        state: "untested",
    },
    ProfileRow {
        vendor: "msi",
        pattern: ".*b550.*",
        profile: "msi-b550-tomahawk",
        state: "untested",
    },
    ProfileRow {
        vendor: "panasonic",
        pattern: ".*cf-lx3.*",
        profile: "letsnote-cf-lx3",
        state: "untested",
    },
    ProfileRow {
        vendor: "panasonic",
        pattern: ".*cf-lx4.*",
        profile: "letsnote-cf-lx4",
        state: "untested",
    },
    ProfileRow {
        vendor: "purism",
        pattern: ".*librem 13.*",
        profile: "purism-librem-13v3",
        state: "untested",
    },
    ProfileRow {
        vendor: "purism",
        pattern: ".*librem 15.*",
        profile: "purism-librem-15v3",
        state: "untested",
    },
    ProfileRow {
        vendor: "razer",
        pattern: ".*blade 14.*",
        profile: "razer-blade-14-RZ09-0530",
        state: "untested",
    },
    ProfileRow {
        vendor: "samsung",
        pattern: ".*np900x3c.*",
        profile: "samsung-np900x3c",
        state: "unsupported",
    },
    ProfileRow {
        vendor: "slimbook",
        pattern: ".*hero.*",
        profile: "slimbook-hero-rpl-rtx",
        state: "untested",
    },
    ProfileRow {
        vendor: "starlabs",
        pattern: ".*starlite.*",
        profile: "starlabs-starlite-i5",
        state: "untested",
    },
    ProfileRow {
        vendor: "system76",
        pattern: ".*gaze.*",
        profile: "system76-gaze18",
        state: "untested",
    },
    ProfileRow {
        vendor: "system76",
        pattern: ".*galago.*",
        profile: "system76-galp5-1650",
        state: "untested",
    },
    ProfileRow {
        vendor: "system76",
        pattern: ".*darter.*",
        profile: "system76-darp6",
        state: "untested",
    },
    ProfileRow {
        vendor: "system76",
        pattern: ".*thelio.*",
        profile: "system76-thelio-mega",
        state: "untested",
    },
    ProfileRow {
        vendor: "system76",
        pattern: ".*",
        profile: "system76",
        state: "untested",
    },
    ProfileRow {
        vendor: "toshiba",
        pattern: ".*satellite.*",
        profile: "toshiba-swanky",
        state: "unsupported",
    },
    ProfileRow {
        vendor: "tuxedo",
        pattern: ".*aura 15.*",
        profile: "tuxedo-aura-15-gen1",
        state: "untested",
    },
    ProfileRow {
        vendor: "tuxedo",
        pattern: ".*infinitybook.*14.*gen 9.*intel.*",
        profile: "tuxedo-infinitybook-pro14-gen9-intel",
        state: "untested",
    },
    ProfileRow {
        vendor: "tuxedo",
        pattern: ".*infinitybook.*14.*gen 9.*amd.*",
        profile: "tuxedo-infinitybook-pro14-gen9-amd",
        state: "untested",
    },
    ProfileRow {
        vendor: "tuxedo",
        pattern: ".*infinitybook.*14.*gen 10.*amd.*",
        profile: "tuxedo-infinitybook-pro14-gen10-amd",
        state: "untested",
    },
    ProfileRow {
        vendor: "tuxedo",
        pattern: ".*infinitybook.*15.*gen 10.*amd.*",
        profile: "tuxedo-infinitybook-pro15-gen10-amd",
        state: "untested",
    },
    ProfileRow {
        vendor: "tuxedo",
        pattern: ".*pulse 14.*",
        profile: "tuxedo-pulse-14-gen3",
        state: "untested",
    },
    ProfileRow {
        vendor: "tuxedo",
        pattern: ".*pulse 15.*",
        profile: "tuxedo-pulse-15-gen2",
        state: "untested",
    },
    ProfileRow {
        vendor: "xiaomi",
        pattern: ".*redmibook 15.*",
        profile: "xiaomi-redmibook-15-pro-2021",
        state: "untested",
    },
    ProfileRow {
        vendor: "xiaomi",
        pattern: ".*redmibook 16.*",
        profile: "xiaomi-redmibook-16-pro-2024",
        state: "untested",
    },
];

pub(crate) struct GpuVariant {
    pub(crate) layout: &'static str,
    pub(crate) profile: Option<&'static str>,
}

pub(crate) struct GpuMeta {
    pub(crate) profile: &'static str,
    pub(crate) owns: bool,
    pub(crate) variants: &'static [GpuVariant],
}

pub(crate) const GPU_PROFILES: &[GpuMeta] = &[
    GpuMeta {
        profile: "asus-ally-rc71l",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-flow-gv302x-amdgpu",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "amd",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-amd-nvidia",
                profile: Some("asus-flow-gv302x-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "asus-flow-gz301vu",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-intel-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-proart-px13",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-proart-px13-hn7306eac",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-rog-gl552vw",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-intel-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-rog-strix-g513im",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-amd-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-rog-strix-g533q",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-amd-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-rog-strix-g533zw",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-intel-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-rog-strix-g713ie",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-amd-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-rog-strix-g733qs",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-amd-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-zephyrus-ga401",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-amd-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-zephyrus-ga401iv",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-amd-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-zephyrus-ga402",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-zephyrus-ga402x-amdgpu",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "amd",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-amd-nvidia",
                profile: Some("asus-zephyrus-ga402x-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "asus-zephyrus-ga502",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-amd-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-zephyrus-ga503",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-amd-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-zephyrus-gu603h",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-intel-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-zephyrus-gu605cw",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-intel-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "asus-zephyrus-gu605my",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-intel-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "dell-xps-15-7590",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "intel",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-intel-nvidia",
                profile: Some("dell-xps-15-7590-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "dell-xps-15-9500",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "intel",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-intel-nvidia",
                profile: Some("dell-xps-15-9500-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "dell-xps-15-9510",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "intel",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-intel-nvidia",
                profile: Some("dell-xps-15-9510-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "dell-xps-15-9520",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "intel",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-intel-nvidia",
                profile: Some("dell-xps-15-9520-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "dell-xps-15-9530",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "intel",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-intel-nvidia",
                profile: Some("dell-xps-15-9530-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "dell-xps-15-9550",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "intel",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-intel-nvidia",
                profile: Some("dell-xps-15-9550-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "dell-xps-15-9560-nvidia",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "hybrid-intel-nvidia",
                profile: None,
            },
            GpuVariant {
                layout: "intel",
                profile: Some("dell-xps-15-9560-intel"),
            },
        ],
    },
    GpuMeta {
        profile: "dell-xps-15-9570-nvidia",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "hybrid-intel-nvidia",
                profile: None,
            },
            GpuVariant {
                layout: "intel",
                profile: Some("dell-xps-15-9570-intel"),
            },
        ],
    },
    GpuMeta {
        profile: "dell-xps-17-9700-intel",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "intel",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-intel-nvidia",
                profile: Some("dell-xps-17-9700-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "dell-xps-17-9710-intel",
        owns: true,
        variants: &[GpuVariant {
            layout: "intel",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "framework-13-7040-amd",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "framework-16-7040-amd",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "framework-16-amd-ai-300-series",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "amd",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-amd-nvidia",
                profile: Some("framework-16-amd-ai-300-series-nvidia"),
            },
        ],
    },
    GpuMeta {
        profile: "framework-desktop-amd-ai-max-300-series",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "framework-amd-ai-300-series",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "hp-elitebook-845g7",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "hp-elitebook-845g8",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "hp-elitebook-845g9",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "lenovo-ideapad-15ach6",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "lenovo-legion-15ach6h",
        owns: true,
        variants: &[GpuVariant {
            layout: "hybrid-amd-nvidia",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "lenovo-yoga-6-13ALC6",
        owns: true,
        variants: &[GpuVariant {
            layout: "amd",
            profile: None,
        }],
    },
    GpuMeta {
        profile: "lenovo-yoga-7-14ARH7-amdgpu",
        owns: true,
        variants: &[
            GpuVariant {
                layout: "amd",
                profile: None,
            },
            GpuVariant {
                layout: "hybrid-amd-nvidia",
                profile: Some("lenovo-yoga-7-14ARH7-nvidia"),
            },
        ],
    },
];

pub(crate) const VENDOR_ALIASES: &[(&str, &str)] = &[
    ("apple", ".*apple.*"),
    ("asus", ".*asus.*"),
    ("asus", ".*asustek.*"),
    ("chuwi", ".*chuwi.*"),
    ("dell", ".*dell.*"),
    ("framework", ".*framework.*"),
    ("gigabyte", ".*gigabyte.*"),
    ("google", ".*google.*"),
    ("gpd", ".*gpd.*"),
    ("hp", ".*hewlett.*packard.*"),
    ("hp", ".*hp.*"),
    ("hp", ".*omen.*"),
    ("huawei", ".*huawei.*"),
    ("intel", ".*intel.*"),
    ("lenovo", ".*lenovo.*"),
    ("microsoft", ".*microsoft.*"),
    ("msi", ".*micro.star.*"),
    ("msi", ".*msi.*"),
    ("panasonic", ".*panasonic.*"),
    ("purism", ".*purism.*"),
    ("razer", ".*razer.*"),
    ("samsung", ".*samsung.*"),
    ("slimbook", ".*slimbook.*"),
    ("starlabs", ".*star.?labs.*"),
    ("system76", ".*system76.*"),
    ("toshiba", ".*toshiba.*"),
    ("toshiba", ".*dynabook.*"),
    ("tuxedo", ".*tuxedo.*"),
    ("xiaomi", ".*xiaomi.*"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuDetection {
    pub layout: String,
    pub source: String,
    pub managed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareDetection {
    pub model: String,
    pub vendor: String,
    pub base_profile: String,
    pub profile: String,
    pub state: String,
    pub gpu: GpuDetection,
}

impl HardwareDetection {
    pub fn empty() -> Self {
        Self {
            model: "unknown".to_string(),
            vendor: "unknown".to_string(),
            base_profile: "unmatched".to_string(),
            profile: "unmatched".to_string(),
            state: "unmatched".to_string(),
            gpu: GpuDetection {
                layout: "unknown".to_string(),
                source: "unknown".to_string(),
                managed: false,
            },
        }
    }
}

pub fn detect_report(text: &str) -> Option<HardwareDetection> {
    let report: serde_json::Value = serde_json::from_str(text).ok()?;
    Some(detect(&report))
}

pub(crate) fn detect(report: &serde_json::Value) -> HardwareDetection {
    let mut detection = HardwareDetection::empty();
    let system = report.pointer("/smbios/system");
    let manufacturer = json_str(system, "manufacturer");
    let product = json_str(system, "product");
    let version = json_str(system, "version");
    let model = [manufacturer.as_str(), product.as_str()]
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !model.is_empty() {
        detection.model = model;
    }
    detection.vendor = classify_vendor(&manufacturer).to_string();
    let lowered_product = product.to_lowercase();
    let lowered_version = version.to_lowercase();
    if detection.vendor != "unknown" {
        if let Some(row) = HARDWARE_PROFILES.iter().find(|row| {
            row.vendor == detection.vendor.as_str()
                && (full_match(row.pattern, &lowered_product)
                    || full_match(row.pattern, &lowered_version))
        }) {
            detection.base_profile = row.profile.to_string();
            detection.profile = row.profile.to_string();
            detection.state = row.state.to_string();
        }
    }
    let gpu = detect_gpu(report);
    let (profile, gpu) = resolve_gpu_profile(&detection.base_profile, &gpu);
    detection.profile = profile;
    detection.gpu = gpu;
    detection
}

/// Every profile the widget can select, sorted for a stable dropdown.
pub fn profile_choices() -> Vec<String> {
    let mut names: Vec<String> = HARDWARE_PROFILES
        .iter()
        .map(|row| row.profile.to_string())
        .chain(GPU_PROFILES.iter().flat_map(|meta| {
            std::iter::once(meta.profile.to_string()).chain(
                meta.variants
                    .iter()
                    .filter_map(|variant| variant.profile.map(str::to_string)),
            )
        }))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The profile and GPU state the configuration would use for `selected`, the
/// raw value of `ojuny.hardware.profile`. `auto` follows the report; an
/// explicit profile follows its entry in the GPU table.
pub(crate) fn effective(selected: &str, detection: &HardwareDetection) -> (String, GpuDetection) {
    if selected == "auto" {
        return (detection.profile.clone(), detection.gpu.clone());
    }
    let base = gpu_profile_base(selected).unwrap_or(selected);
    match GPU_PROFILES.iter().find(|meta| meta.profile == base) {
        Some(meta) => {
            let variant = meta
                .variants
                .iter()
                .find(|variant| variant.profile == Some(selected))
                .or_else(|| meta.variants.first());
            let layout = variant
                .map(|variant| variant.layout)
                .unwrap_or(detection.gpu.layout.as_str())
                .to_string();
            (
                selected.to_string(),
                GpuDetection {
                    layout,
                    source: "profile".to_string(),
                    managed: meta.owns,
                },
            )
        }
        None => (
            selected.to_string(),
            GpuDetection {
                layout: detection.gpu.layout.clone(),
                source: detection.gpu.source.clone(),
                managed: false,
            },
        ),
    }
}

fn gpu_profile_base(selected: &str) -> Option<&'static str> {
    GPU_PROFILES.iter().find_map(|meta| {
        meta.variants
            .iter()
            .any(|variant| variant.profile == Some(selected))
            .then_some(meta.profile)
    })
}

fn detect_gpu(report: &serde_json::Value) -> GpuDetection {
    let hardware = report.get("hardware");
    let present = hardware
        .and_then(serde_json::Value::as_object)
        .map(|hardware| hardware.contains_key("graphics_card"))
        .unwrap_or(false);
    let cards: Vec<&serde_json::Value> = hardware
        .and_then(|hardware| hardware.get("graphics_card"))
        .and_then(serde_json::Value::as_array)
        .map(|cards| cards.iter().collect())
        .unwrap_or_default();
    let intel: Vec<&serde_json::Value> = cards
        .iter()
        .copied()
        .filter(|card| is_intel(card))
        .collect();
    let amd: Vec<&serde_json::Value> = cards.iter().copied().filter(|card| is_amd(card)).collect();
    let nvidia: Vec<&serde_json::Value> = cards
        .iter()
        .copied()
        .filter(|card| is_nvidia(card))
        .collect();
    let layout = if intel.is_empty() && amd.is_empty() && nvidia.is_empty() {
        "unknown"
    } else if !intel.is_empty() && !nvidia.is_empty() && amd.is_empty() {
        "hybrid-intel-nvidia"
    } else if !amd.is_empty() && !nvidia.is_empty() && intel.is_empty() {
        "hybrid-amd-nvidia"
    } else if !intel.is_empty() && !amd.is_empty() && nvidia.is_empty() {
        "hybrid-intel-amd"
    } else if !intel.is_empty() && amd.is_empty() && nvidia.is_empty() {
        "intel"
    } else if !amd.is_empty() && intel.is_empty() && nvidia.is_empty() {
        "amd"
    } else if !nvidia.is_empty() && intel.is_empty() && amd.is_empty() {
        "nvidia"
    } else {
        "unknown"
    };
    GpuDetection {
        layout: layout.to_string(),
        source: if present { "facter" } else { "unknown" }.to_string(),
        managed: false,
    }
}

fn resolve_gpu_profile(base: &str, gpu: &GpuDetection) -> (String, GpuDetection) {
    let Some(meta) = GPU_PROFILES.iter().find(|meta| meta.profile == base) else {
        return (base.to_string(), gpu.clone());
    };
    let variant = meta
        .variants
        .iter()
        .find(|variant| variant.layout == gpu.layout)
        .or_else(|| meta.variants.first());
    let profile = variant
        .and_then(|variant| variant.profile)
        .unwrap_or(base)
        .to_string();
    let layout = variant
        .map(|variant| variant.layout)
        .unwrap_or(gpu.layout.as_str())
        .to_string();
    (
        profile,
        GpuDetection {
            layout,
            source: gpu.source.clone(),
            managed: meta.owns,
        },
    )
}

fn classify_vendor(manufacturer: &str) -> &'static str {
    let lowered = manufacturer.to_lowercase();
    VENDOR_ALIASES
        .iter()
        .find(|(_, pattern)| full_match(pattern, &lowered))
        .map(|(vendor, _)| *vendor)
        .unwrap_or("unknown")
}

fn full_match(pattern: &str, text: &str) -> bool {
    regex::Regex::new(&format!("^(?:{pattern})$"))
        .map(|regex| regex.is_match(text))
        .unwrap_or(false)
}

fn json_str(value: Option<&serde_json::Value>, key: &str) -> String {
    value
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn card_vendor(card: &serde_json::Value) -> (String, String) {
    let name = card
        .pointer("/vendor/name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    let hex = card
        .pointer("/vendor/hex")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    (name, hex)
}

fn is_intel(card: &serde_json::Value) -> bool {
    let (name, hex) = card_vendor(card);
    name.contains("intel") || hex == "8086"
}

fn is_amd(card: &serde_json::Value) -> bool {
    let (name, hex) = card_vendor(card);
    matches!(hex.as_str(), "1002" | "1022")
        || name.contains("advanced micro devices")
        || name.contains("ati ")
        || name == "ati"
}

fn is_nvidia(card: &serde_json::Value) -> bool {
    let (name, hex) = card_vendor(card);
    name.contains("nvidia") || hex == "10de"
}

#[cfg(test)]
mod tests {
    use super::*;

    const SURFACE_GO: &str = r#"{
        "smbios": {
            "system": {
                "manufacturer": "Microsoft Corporation",
                "product": "Surface Go",
                "version": "1.0"
            }
        }
    }"#;

    #[test]
    fn detects_a_surface_go_from_the_report() {
        let detection = detect_report(SURFACE_GO).unwrap();
        assert_eq!(detection.model, "Microsoft Corporation Surface Go");
        assert_eq!(detection.vendor, "microsoft");
        assert_eq!(detection.profile, "microsoft-surface-go");
        assert_eq!(detection.state, "untested");
        assert_eq!(detection.gpu.layout, "unknown");
        assert_eq!(detection.gpu.source, "unknown");
        assert!(!detection.gpu.managed);
    }

    #[test]
    fn detects_a_hybrid_gpu_and_its_variant_profile() {
        let report = serde_json::json!({
            "smbios": {
                "system": {
                    "manufacturer": "Dell Inc.",
                    "product": "XPS 15 9520"
                }
            },
            "hardware": {
                "graphics_card": [
                    {"vendor": {"name": "Intel Corporation", "hex": "8086"}},
                    {"vendor": {"name": "NVIDIA Corporation", "hex": "10de"}}
                ]
            }
        });
        let detection = detect(&report);
        assert_eq!(detection.profile, "dell-xps-15-9520-nvidia");
        assert_eq!(detection.gpu.layout, "hybrid-intel-nvidia");
        assert_eq!(detection.gpu.source, "facter");
        assert!(detection.gpu.managed);
    }

    #[test]
    fn an_explicit_profile_reports_its_effective_gpu_state() {
        let detection = detect_report(SURFACE_GO).unwrap();
        let (profile, gpu) = effective("dell-xps-15-9520-nvidia", &detection);
        assert_eq!(profile, "dell-xps-15-9520-nvidia");
        assert_eq!(gpu.layout, "hybrid-intel-nvidia");
        assert_eq!(gpu.source, "profile");
        assert!(gpu.managed);

        let (profile, gpu) = effective("custom-profile", &detection);
        assert_eq!(profile, "custom-profile");
        assert_eq!(gpu.layout, "unknown");
        assert_eq!(gpu.source, "unknown");
        assert!(!gpu.managed);
    }

    #[test]
    fn an_unmatched_machine_stays_unmatched() {
        let report = serde_json::json!({
            "smbios": {
                "system": {
                    "manufacturer": "Acme",
                    "product": "Unknown 1"
                }
            }
        });
        let detection = detect(&report);
        assert_eq!(detection.profile, "unmatched");
        assert_eq!(detection.state, "unmatched");
    }
}
