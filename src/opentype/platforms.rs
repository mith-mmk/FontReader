use std::default;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u16)]
pub enum PlatformID {
    Unicode = 0,
    Macintosh = 1,
    ISO = 2, // deprecated
    Windows =3,
    Custom = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum EncordingId {
    UnicodeEncordingId(UnicodeEncordingId),
    WindowsEndoridingId(WindowsEndoridingId),
    MacintoshEncordingID(MacintoshEncordingID),
    Unknown(u16),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum LanguageID {
    UnicodeLanguageID(u16),
    WindowsLanguageID(WindowsLanguageID),
    MacintoshLanguageID(MacintoshLanguageID),
    Unknown(u16),
}


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u16)]
pub enum UnicodeEncordingId {
    Unicode1_0 = 0, // deprecated
    Unicode1_1 = 1, // deprecated
    ISO10646 = 2, // deprecated
    Unicode2_0BMP = 3,
    Unicode2_0Full = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u16)]
pub enum WindowsEndoridingId {
    Symbol = 0,
    UnicodeBMP = 1,
    ShiftJIS = 2,
    PRC = 3,
    Big5 = 4,
    Wansung = 5,
    Johab = 6,
    UnicodeFullRepertoire = 10,   
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u16)]
pub enum MacintoshEncordingID {
    Roman = 0,
    Japanese = 1,
    TraditionalChinese = 2,
    Korean = 3,
    Arabic = 4,
    Hebrew = 5,
    Greek = 6,
    Russian = 7,
    RSymbol = 8,
    Devanagari = 9,
    Gurmukhi = 10,
    Gujarati = 11,
    Oriya = 12,
    Bengali = 13,
    Tamil = 14,
    Telugu = 15,
    Kannada = 16,
    Malayalam = 17,
    Sinhalese = 18,
    Burmese = 19,
    Khmer = 20,
    Thai = 21,
    Laotian = 22,
    Georgian = 23,
    Armenian = 24,
    ChineseSimplified = 25,
    Tibetan = 26,
    Mongolian = 27,
    Geez = 28,
    Slavic = 29,
    Vietnamese = 30,
    Sindhi = 31,
    Uninterpreted = 32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u16)]
pub enum MacintoshLanguageID {
    English = 0,
    French = 1,
    German = 2,
    Italian = 3,
    Dutch = 4,
    Swedish = 5,
    Spanish = 6,
    Danish = 7,
    Portuguese = 8,
    Norwegian = 9,
    Hebrew = 10,
    Japanese = 11,
    Arabic = 12,
    Finnish = 13,
    Greek = 14,
    Icelandic = 15,
    Maltese = 16,
    Turkish = 17,
    Croatian = 18,
    ChineseTraditional = 19,
    Urdu = 20,
    Hindi = 21,
    Thai = 22,
    Korean = 23,
    Lithuanian = 24,
    Polish = 25,
    Hungarian = 26,
    Estonian = 27,
    Latvian = 28,
    Sami = 29,
    Faroese = 30,
    FarsiPersian = 31,
    Russian = 32,
    ChineseSimplified = 33,
    Flemish = 34,
    IrishGaelic = 35,
    Albanian = 36,
    Romanian = 37,
    Czech = 38,
    Slovak = 39,
    Slovenian = 40,
    Yiddish = 41,
    Serbian = 42,
    Macedonian = 43,
    Bulgarian = 44,
    Ukrainian = 45,
    Byelorussian = 46,
    Uzbek = 47,
    Kazakh = 48,
    AzerbaijaniCyrillicScript = 49,
    AzerbaijaniArabicScript = 50,
    Armenian = 51,
    Georgian = 52,
    Moldavian = 53,
    Kirghiz = 54,
    Tajiki = 55,
    Turkmen = 56,
    MongolianMongolianScript = 57,
    MongolianCyrillicScript = 58,
    Pashto = 59,
    Kurdish = 60,
    Kashmiri = 61,
    Sindhi = 62,
    Tibetan = 63,
    Nepali = 64,
    Sanskrit = 65,
    Marathi = 66,
    Bengali = 67,
    Assamese = 68,
    Gujarati = 69,
    Punjabi = 70,
    Oriya = 71,
    Malayalam = 72,
    Kannada = 73,
    Tamil = 74,
    Telugu = 75,
    Sinhalese = 76,
    Burmese = 77,
    Khmer = 78,
    Lao = 79,
    Vietnamese = 80,
    Indonesian = 81,
    Tagalog = 82,
    MalayRomanScript = 83,
    MalayArabicScript = 84,
    Amharic = 85,
    Tigrinya = 86,
    Galla = 87,
    Somali = 88,
    Swahili = 89,
    KinyarwandaRuanda = 90,
    Rundi = 91,
    NyanjaChewa = 92,
    Malagasy = 93,
    Esperanto = 94,
    Welsh = 128,
    Basque = 129,
    Catalan = 130,
    Latin = 131,
    Quenchua = 132,
    Guarani = 133,
    Aymara = 134,
    Tatar = 135,
    Uighur = 136,
    Dzongkha = 137,
    JavaneseRomanScript = 138,
    SundaneseRomanScript = 139,
    Galician = 140,
    Afrikaans = 141,
    Breton = 142,
    Inuktitut = 143,
    ScottishGaelic = 144,
    ManxGaelic = 145,
    IrishGaelicwithdotabove = 146,
    Tongan = 147,
    Greekpolytonic = 148,
    Greenlandic = 149,
    AzerbaijaniRomanScript = 150,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u16)]
pub enum WindowsLanguageID {
    AfrikaansSouthAfrica = 0x0436,
    AlbanianAlbania = 0x041c,
    AlsatianFrance = 0x0484,
    AmharicEthiopia = 0x045e,
    ArabicAlgeria = 0x1401,
    ArabicBahrain = 0x3c01,
    ArabicEgypt = 0x0c01,
    ArabicIraq = 0x0801,
    ArabicJordan = 0x2c01,
    ArabicKuwait = 0x3401,
    ArabicLebanon = 0x3001,
    ArabicLibya = 0x1001,
    ArabicMorocco = 0x1801,
    ArabicOman = 0x2001,
    ArabicQatar = 0x4001,
    ArabicSaudiArabia = 0x0401,
    ArabicSyria = 0x2801,
    ArabicTunisia = 0x1c01,
    ArabicUAE = 0x3801,
    ArabicYemen = 0x2401,
    Armenian = 0x042b,
    Assamese = 0x044d,
    AzeriCyrillic = 0x082c,
    AzeriLatin = 0x042c,
    Bashkir = 0x046d,
    Basque = 0x042d,
    Belarusian = 0x0423,
    BengaliBangladesh = 0x0845,
    BengaliIndia = 0x0445,
    BosnianCyrillic = 0x201a,
    BosnianLatin = 0x141a,
    Breton = 0x047e,
    Bulgarian = 0x0402,
    Catalan = 0x0403,
    ChineseHongKongSAR = 0x0c04,
    ChineseMacaoSAR = 0x1404,
    ChinesePRC = 0x0804,
    ChineseSingapore = 0x1004,
    ChineseTaiwan = 0x0404,
    Corsican = 0x0483,
    Croatian = 0x041a,
    CroatianBosniaHerzegovina = 0x101a,
    Czech = 0x0405,
    Danish = 0x0406,
    Dari = 0x048c,
    Divehi = 0x0465,
    DutchBelgium = 0x0813,
    DutchNetherlands = 0x0413,
    Edo = 0x0466,
    EnglishAustralia = 0x0c09,
    EnglishBelize = 0x2809,
    EnglishCanada = 0x1009,
    EnglishCaribbean = 0x2409,
    EnglishIndia = 0x4009,
    EnglishIreland = 0x1809,
    EnglishJamaica = 0x2009,
    EnglishMalaysia = 0x4409,
    EnglishNewZealand = 0x1409,
    EnglishPhilippines = 0x3409,
    EnglishSingapore = 0x4809,
    EnglishSouthAfrica = 0x1c09,
    EnglishTrinidadTobago = 0x2c09,
    EnglishUnitedKingdom = 0x0809,
    EnglishUnitedStates = 0x0409,
    EnglishZimbabwe = 0x3009,
    Estonian = 0x0425,
    Faroese = 0x0438,
    Filipino = 0x0464,
    Finnish = 0x040b,
    FrenchBelgium = 0x080c,
    FrenchCanada = 0x0c0c,
    FrenchFrance = 0x040c,
    FrenchLuxembourg = 0x140c,
    FrenchMonaco = 0x180c,
    FrenchSwitzerland = 0x100c,
    FrisianNetherlands = 0x0462,
    Fulfulde = 0x0467,
    Galician = 0x0456,
    Georgian = 0x0437,
    GermanAustria = 0x0c07,
    GermanGermany = 0x0407,
    GermanLiechtenstein = 0x1407,
    GermanLuxembourg = 0x1007,
    GermanSwitzerland = 0x0807,
    Greek = 0x0408,
    Greenlandic = 0x046f,
    Guarani = 0x0474,
    Gujarati = 0x0447,
    Hausa = 0x0468,
    Hawaiian = 0x0475,
    Hebrew = 0x040d,
    Hindi = 0x0439,
    Hungarian = 0x040e,
    Ibibio = 0x0469,
    Icelandic = 0x040f,
    Igbo = 0x0470,
    Indonesian = 0x0421,
    Inuktitut = 0x045d,
    Irish = 0x083c,
    isiXhosa = 0x0434,
    isiZulu = 0x0435,
    ItalianItaly = 0x0410,
    ItalianSwitzerland = 0x0810,
    Japanese = 0x0411,
    Kannada = 0x044b,
    Kiche = 0x0486,
    Kinyarwanda = 0x0487,
    Kiswahili = 0x0441,
    Konkani = 0x0457,
    Korean = 0x0412,
    Kyrgyz = 0x0440,
    Lao = 0x0454,
    Latin = 0x0476,
    Latvian = 0x0426,
    Lithuanian = 0x0427,
    Luxembourgish = 0x046e,
    Macedonian = 0x042f,
    MalayBruneiDarussalam = 0x083e,
    MalayMalaysia = 0x043e,
    Malayalam = 0x044c,
    Maltese = 0x043a,
    Manipuri = 0x0458,
    Maori = 0x0481,
    Mapudungun = 0x047a,
    Marathi = 0x044e,
    Mohawk = 0x047c,
    MongolianCyrillic = 0x0450,
    MongolianTraditional = 0x0850,
    Nepali = 0x0461,
    NorwegianBokmål = 0x0414,
    NorwegianNynorsk = 0x0814,
    Occitan = 0x0482,
    Oriya = 0x0448,
    Pashto = 0x0463,
    Polish = 0x0415,
    PortugueseBrazil = 0x0416,
    PortuguesePortugal = 0x0816,
    Punjabi = 0x0446,
    QuechuaBolivia = 0x046b,
    QuechuaEcuador = 0x086b,
    QuechuaPeru = 0x0c6b,
    RhaetoRomanic = 0x0417,
    Romanian = 0x0418,
    RomanianMoldava = 0x0818,
    Russian = 0x0419,
    RussianMoldava = 0x0819,
    SamiInariFinland = 0x243b,
    SamiLuleNorway = 0x103b,
    SamiLuleSweden = 0x143b,
    SamiNorthernFinland = 0x0c3b,
    SamiNorthernNorway = 0x043b,
    SamiNorthernSweden = 0x083b,
    SamiSkoltFinland = 0x203b,
    SamiSouthernNorway = 0x183b,
    SamiSouthernSweden = 0x1c3b,
    Sanskrit = 0x044f,
    SerbianCyrillic = 0x0c1a,
    SerbianLatin = 0x081a,
    SesothoSaLeboa = 0x046c,
    Setswana = 0x0432,
    Sinhalese = 0x045b,
    Slovak = 0x041b,
    Slovenian = 0x0424,
    Somali = 0x0477,
    Sorbian = 0x042e,
    SpanishArgentina = 0x2c0a,
    SpanishBolivia = 0x400a,
    SpanishChile = 0x340a,
    SpanishColombia = 0x240a,
    SpanishCostaRica = 0x140a,
    SpanishDominicanRepublic = 0x1c0a,
    SpanishEcuador = 0x300a,
    SpanishElSalvador = 0x440a,
    SpanishGuatemala = 0x100a,
    SpanishHonduras = 0x480a,
    SpanishMexico = 0x080a,
    SpanishNicaragua = 0x4c0a,
    SpanishPanama = 0x180a,
    SpanishParaguay = 0x3c0a,
    SpanishPeru = 0x280a,
    SpanishPuertoRico = 0x500a,
    SpanishSpain = 0x0c0a,
    SpanishTraditionalSort = 0x040a,
    SpanishUruguay = 0x380a,
    SpanishVenezuela = 0x200a,
    Sutu = 0x0430,
    SwedishFinland = 0x081d,
    SwedishSweden = 0x041d,
    Syriac = 0x045a,
    Tajik = 0x0428,
    Tamazight = 0x045f,
    TamazightLatin = 0x085f,
    Tamil = 0x0449,
    Tatar = 0x0444,
    Telugu = 0x044a,
    Thai = 0x041e,
    TibetanBhutan = 0x0851,
    TibetanPRC = 0x0451,
    Turkish = 0x041f,
    Turkmen = 0x0442,
    Uighur = 0x0480,
    Ukrainian = 0x0422,
    Urdu = 0x0420,
    UrduIndia = 0x0820,
    UzbekCyrillic = 0x0843,
    UzbekLatin = 0x0443,
    Venda = 0x0433,
    Vietnamese = 0x042a,
    Welsh = 0x0452,
    Wolof = 0x0488,
    Yakut = 0x0485,
    Yi = 0x0478,
    Yoruba = 0x046a,
    KnownLanguage = 0x0000,
}

/// Ja or Ja_JP, Windows -> Some(0x0411)
pub fn get_locale_to_language_id(locale: &String, platform_id: PlatformID) -> Option<u16> {
  let binding = locale.to_uppercase();
  let binding = binding.split(".").collect::<Vec<&str>>()[0].replace('_', "-");
  let locale:Vec<&str> = binding.split("-").collect();
  let primary_language = locale[0];
  let extended_language = if locale.len() > 1 {
    Some(locale[1])
  } else {
    None
  };

  match platform_id {
    PlatformID::Unicode => {
      None
    },
    PlatformID::Macintosh => {
      let default =MacintoshLanguageID::English;
      match primary_language {
        "C" => {
          Some(MacintoshLanguageID::English as u16)
        }
        "POSIX" => {
          Some(MacintoshLanguageID::English as u16)
        }
        "AF" => {
          if let Some(extended_language) = extended_language {
            match extended_language {
              "ZA" => {
                Some(MacintoshLanguageID::Afrikaans as u16)
              }              
              _ => {
                Some(default as u16)
              }
            }
          } else {
            Some(default as u16)
          }
        }
        "AR" => {
            Some(MacintoshLanguageID::Arabic as u16)
        }
        "AS" => {
          Some(MacintoshLanguageID::Assamese as u16)
        }
        "EU" => {
          Some(MacintoshLanguageID::Basque as u16)
        }
        "BE" => {
          Some(default as u16)
        }
        "BN" => {
          Some(MacintoshLanguageID::Bengali as u16)
        }
        "BG" => {
          Some(MacintoshLanguageID::Bulgarian as u16)
        }
        "CA" => {
          Some(MacintoshLanguageID::Catalan as u16)
        }
        "ZH" => {
          if let Some(extended_language) = extended_language {
            match extended_language {
              "CN" => {
                Some(MacintoshLanguageID::ChineseSimplified as u16)
              }
              "HK" => {
                Some(MacintoshLanguageID::ChineseTraditional as u16)
              }
              "MO" => {
                Some(MacintoshLanguageID::ChineseTraditional as u16)
              }
              "SG" => {
                Some(MacintoshLanguageID::ChineseSimplified as u16)
              }
              "TW" => {
                Some(MacintoshLanguageID::ChineseTraditional as u16)
              }
              _ => {
                Some(MacintoshLanguageID::ChineseSimplified as u16)
              }
            }
          } else {
            Some(MacintoshLanguageID::ChineseSimplified as u16)
          }
        }
        "HR" => {
          Some(MacintoshLanguageID::Croatian as u16)
        }
        "CS" => {
          Some(MacintoshLanguageID::Czech as u16)
        }
        "DA" => {
          Some(MacintoshLanguageID::Danish as u16)
        }
        "NL" => {
          Some(MacintoshLanguageID::Dutch as u16)
        }
        "EN" => {
          Some(MacintoshLanguageID::English as u16)
        }
        "ET" => {
          Some(MacintoshLanguageID::Estonian as u16)
        }
        "FO" => {
          Some(MacintoshLanguageID::Faroese as u16)
        }
        "FI" => {
          Some(MacintoshLanguageID::Finnish as u16)
        }
        "FR" => {
          Some(MacintoshLanguageID::French as u16)
        }
        "GD" => {
          Some(default as u16)
        }
        "GL" => {
          Some(MacintoshLanguageID::Galician as u16)
        }
        "JA" => {
          Some(MacintoshLanguageID::Japanese as u16)
        }
        "KA" => {
          Some(MacintoshLanguageID::Georgian as u16)
        }
        "DE" => {
          Some(MacintoshLanguageID::German as u16)
        }
        "EL" => {
          Some(MacintoshLanguageID::Greek as u16)
        }
        "GU" => {
          Some(MacintoshLanguageID::Gujarati as u16)
        }
        "HE" => {
          Some(MacintoshLanguageID::Hebrew as u16)
        }
        "HI" => {
          Some(MacintoshLanguageID::Hindi as u16)
        }
        "HU" => {
          Some(MacintoshLanguageID::Hungarian as u16)
        }
        "IS" => {
          Some(MacintoshLanguageID::Icelandic as u16)
        }
        "ID" => {
          Some(MacintoshLanguageID::Indonesian as u16)
        }
        "GA" => {
          Some(default as u16)
        }
        "IT" => {
          Some(MacintoshLanguageID::Italian as u16)
        }
        "KN" => {
          Some(MacintoshLanguageID::Kannada as u16)
        }
        "KK" => {
          Some(MacintoshLanguageID::Kazakh as u16)
        }
        "KM" => {
          Some(MacintoshLanguageID::Khmer as u16)
        }
        "KO" => {
          Some(MacintoshLanguageID::Korean as u16)
        }
        "LA" => {
          Some(MacintoshLanguageID::Latin as u16)
        }
        "LV" => {
          Some(MacintoshLanguageID::Latvian as u16)
        }
        "LT" => {
          Some(MacintoshLanguageID::Lithuanian as u16)
        }
        "MS" => {
          Some(default as u16)
        }
        "ML" => {
          Some(MacintoshLanguageID::Malayalam as u16)
        }
        "MT" => {
          Some(MacintoshLanguageID::Maltese as u16)
        }
        "MR" => {
          Some(MacintoshLanguageID::Marathi as u16)
        }
        "MN" => {
          Some(default as u16)
        }
        "NE" => {
          Some(MacintoshLanguageID::Nepali as u16)
        }
        "NO" => {
          Some(MacintoshLanguageID::Norwegian as u16)
        }
        "OR" => {
          Some(MacintoshLanguageID::Oriya as u16)
        }
        "PL" => {
          Some(MacintoshLanguageID::Polish as u16)
        }
        "PT" => {
          Some(MacintoshLanguageID::Portuguese as u16)
        }
        "PA" => {
          Some(MacintoshLanguageID::Punjabi as u16)
        }
        "RM" => {
          Some(default as u16)
        }
        "RO" => {
          Some(MacintoshLanguageID::Romanian as u16)
        }
        "RU" => {
          Some(MacintoshLanguageID::Russian as u16)
        }
        "SA" => {
          Some(MacintoshLanguageID::Sanskrit as u16)
        }
        "SR" => {
          Some(MacintoshLanguageID::Serbian as u16)
        }
        "SK" => {
          Some(MacintoshLanguageID::Slovak as u16)
        }
        "SL" => {
          Some(MacintoshLanguageID::Slovenian as u16)
        }
        "ES" => {
          Some(MacintoshLanguageID::Spanish as u16)
        }
        "SW" => {
          Some(MacintoshLanguageID::Swahili as u16)
        }
        "SV" => {
          Some(MacintoshLanguageID::Swedish as u16)
        }
        "TG" => {
          Some(default as u16)
        }
        "TA" => {
          Some(MacintoshLanguageID::Tamil as u16)
        }
        "TT" => {
          Some(MacintoshLanguageID::Tatar as u16)
        }
        "TE" => {
          Some(MacintoshLanguageID::Telugu as u16)
        }
        "TH" => {
          Some(MacintoshLanguageID::Thai as u16)
        }
        "BO" => {
          Some(MacintoshLanguageID::Tibetan as u16)
        }
        "TR" => {
          Some(MacintoshLanguageID::Turkish as u16)
        }
        "UK" => {
          Some(MacintoshLanguageID::Ukrainian as u16)
        }
        "UR" => {
          Some(MacintoshLanguageID::Urdu as u16)
        }
        "UZ" => {
          Some(MacintoshLanguageID::Uzbek as u16)
        }
        "VI" => {
          Some(MacintoshLanguageID::Vietnamese as u16)
        }
        "CY" => {
          Some(MacintoshLanguageID::Welsh as u16)
        }
        "XH" => {
          Some(default as u16)
        }
        "YO" => {
          Some(default as u16)
        }
        "ZA" => {
          Some(default as u16)
        }
        "ZU" => {
          Some(default as u16)
        }
        _ => {
          Some(default as u16)
        }
      }
    },
    PlatformID::Windows => {
      let default = WindowsLanguageID::EnglishUnitedStates;
      match primary_language {
        "C" => {
          Some(WindowsLanguageID::EnglishUnitedStates as u16)
        }
        "POSIX" => {
          Some(WindowsLanguageID::EnglishUnitedStates as u16)
        }
        "AF" => {
          if let Some(extended_language) = extended_language {
            match extended_language {
              "ZA" => {
                Some(WindowsLanguageID::AfrikaansSouthAfrica as u16)
              }              
              _ => {
                Some(default as u16)
              }
            }
          } else {
            Some(default as u16)
          }
        }
        "AR" => {
            if let Some(extend_language) = extended_language {
              match extend_language {
                "AE" => {
                  Some(WindowsLanguageID::ArabicUAE as u16)
                }
                "BH" => {
                  Some(WindowsLanguageID::ArabicBahrain as u16)
                }
                "DZ" => {
                  Some(WindowsLanguageID::ArabicAlgeria as u16)
                }
                "EG" => {
                  Some(WindowsLanguageID::ArabicEgypt as u16)
                }
                "IQ" => {
                  Some(WindowsLanguageID::ArabicIraq as u16)
                }
                "JO" => {
                  Some(WindowsLanguageID::ArabicJordan as u16)
                }
                "KW" => {
                  Some(WindowsLanguageID::ArabicKuwait as u16)
                }
                "LB" => {
                  Some(WindowsLanguageID::ArabicLebanon as u16)
                }
                "LY" => {
                  Some(WindowsLanguageID::ArabicLibya as u16)
                }
                "MA" => {
                  Some(WindowsLanguageID::ArabicMorocco as u16)
                }
                "OM" => {
                  Some(WindowsLanguageID::ArabicOman as u16)
                }
                "QA" => {
                  Some(WindowsLanguageID::ArabicQatar as u16)
                }
                "SA" => {
                  Some(WindowsLanguageID::ArabicSaudiArabia as u16)
                }
                "SY" => {
                  Some(WindowsLanguageID::ArabicSyria as u16)
                }
                "TN" => {
                  Some(WindowsLanguageID::ArabicTunisia as u16)
                }
                "YE" => {
                  Some(WindowsLanguageID::ArabicYemen as u16)
                }
                _ => {
                  Some(WindowsLanguageID::ArabicUAE as u16)
                }                
              }
            } else {
              Some(WindowsLanguageID::ArabicUAE as u16)
            }
        }
        "AS" => {
          Some(WindowsLanguageID::Assamese as u16)
        }
        "EU" => {
          Some(WindowsLanguageID::Basque as u16)
        }
        "BE" => {
          Some(default as u16)
        }
        "BN" => {
          Some(WindowsLanguageID::BengaliBangladesh as u16)
        }
        "BG" => {
          Some(WindowsLanguageID::Bulgarian as u16)
        }
        "CA" => {
          Some(WindowsLanguageID::Catalan as u16)
        }
        "CS" => {
          Some(WindowsLanguageID::Czech as u16)
        }
        "DA" => {
          Some(WindowsLanguageID::Danish as u16)
        }
        "DE" => {
          if let Some(extend_language) = extended_language {
            match extend_language {
              "AT" => {
                Some(WindowsLanguageID::GermanAustria as u16)
              }
              "BE" => {
                Some(WindowsLanguageID::GermanGermany as u16)
              }
              "CH" => {
                Some(WindowsLanguageID::GermanSwitzerland as u16)
              }
              "LI" => {
                Some(WindowsLanguageID::GermanLiechtenstein as u16)
              }
              "LU" => {
                Some(WindowsLanguageID::GermanLuxembourg as u16)
              }
              _ => {
                Some(WindowsLanguageID::GermanGermany as u16)
              }
            }
          } else {
            Some(WindowsLanguageID::GermanGermany as u16)
          }
        }
        "EL" => {
            Some(WindowsLanguageID::Greek as u16)
        }
        "EN" => {
          if let Some(extend_language) = extended_language {
            match extend_language {
              "AU" => {
                Some(WindowsLanguageID::EnglishAustralia as u16)
              }
              "BW" => {
                Some(WindowsLanguageID::EnglishUnitedStates as u16)
              }
              "BZ" => {
                Some(WindowsLanguageID::EnglishBelize as u16)
              }
              "CA" => {
                Some(WindowsLanguageID::EnglishCanada as u16)
              }
              "CB" => {
                Some(WindowsLanguageID::EnglishCaribbean as u16)
              }
              "GB" => {
                Some(WindowsLanguageID::EnglishUnitedKingdom as u16)
              }
              "HK" => {
                Some(WindowsLanguageID::EnglishUnitedStates as u16)
              }
              "IE" => {
                Some(WindowsLanguageID::EnglishIreland as u16)
              }
              "IN" => {
                Some(WindowsLanguageID::EnglishIndia as u16)
              }
              "MT" => {
                Some(WindowsLanguageID::EnglishMalaysia as u16)
              }
              "JM" => {
                Some(WindowsLanguageID::EnglishJamaica as u16)
              }
              "NZ" => {
                Some(WindowsLanguageID::EnglishNewZealand as u16)
              }
              "PH" => {
                Some(WindowsLanguageID::EnglishPhilippines as u16)
              }
              "SG" => {
                Some(WindowsLanguageID::EnglishSingapore as u16)
              }
              "TT" => {
                Some(WindowsLanguageID::EnglishTrinidadTobago as u16)
              }
              "US" => {
                Some(WindowsLanguageID::EnglishUnitedStates as u16)
              }
              "ZA" => {
                Some(WindowsLanguageID::EnglishSouthAfrica as u16)
              }
              "ZW" => {
                Some(WindowsLanguageID::EnglishZimbabwe as u16)
              }
              _ => {
                Some(WindowsLanguageID::EnglishUnitedStates as u16)
              }
            }
          } else {
            Some(WindowsLanguageID::EnglishUnitedStates as u16)
          }
        }
        "ES" => {
          if let Some(extend_language) = extended_language {
            match extend_language {
              "AR" => {
                Some(WindowsLanguageID::SpanishArgentina as u16)
              }
              "BO" => {
                Some(WindowsLanguageID::SpanishBolivia as u16)
              }
              "CL" => {
                Some(WindowsLanguageID::SpanishChile as u16)
              }
              "CO" => {
                Some(WindowsLanguageID::SpanishColombia as u16)
              }
              "CR" => {
                Some(WindowsLanguageID::SpanishCostaRica as u16)
              }
              "DO" => {
                Some(WindowsLanguageID::SpanishDominicanRepublic as u16)
              }
              "EC" => {
                Some(WindowsLanguageID::SpanishEcuador as u16)
              }
              "ES" => {
                Some(WindowsLanguageID::SpanishSpain as u16)
              }
              "GT" => {
                Some(WindowsLanguageID::SpanishGuatemala as u16)
              }
              "HN" => {
                Some(WindowsLanguageID::SpanishHonduras as u16)
              }
              "MX" => {
                Some(WindowsLanguageID::SpanishMexico as u16)
              }
              "NI" => {
                Some(WindowsLanguageID::SpanishNicaragua as u16)
              }
              "PA" => {
                Some(WindowsLanguageID::SpanishPanama as u16)
              }
              "PE" => {
                Some(WindowsLanguageID::SpanishPeru as u16)
              }
              "PR" => {
                Some(WindowsLanguageID::SpanishPuertoRico as u16)
              }
              "PY" => {
                Some(WindowsLanguageID::SpanishParaguay as u16)
              }
              "SV" => {
                Some(WindowsLanguageID::SpanishElSalvador as u16)
              }
              "UY" => {
                Some(WindowsLanguageID::SpanishUruguay as u16)
              }
              "VE" => {
                Some(WindowsLanguageID::SpanishVenezuela as u16)
              }
              _ => {
                Some(WindowsLanguageID::SpanishSpain as u16)
              }
            }
          } else {
              Some(WindowsLanguageID::SpanishSpain as u16)
          }
        }
        "ET" => {
          Some(WindowsLanguageID::Estonian as u16)
        }
        "FI" => {
          Some(WindowsLanguageID::Finnish as u16)
        }
        "FR" => {
          if let Some(extend_language) = extended_language {
            match extend_language {
              "BE" => {
                Some(WindowsLanguageID::FrenchBelgium as u16)
              }
              "CA" => {
                Some(WindowsLanguageID::FrenchCanada as u16)
              }
              "CH" => {
                Some(WindowsLanguageID::FrenchSwitzerland as u16)
              }
              "FR" => {
                Some(WindowsLanguageID::FrenchFrance as u16)
              }
              "LU" => {
                Some(WindowsLanguageID::FrenchLuxembourg as u16)
              }
              "MC" => {
                Some(WindowsLanguageID::FrenchMonaco as u16)
              }
              _ => {
                Some(WindowsLanguageID::FrenchFrance as u16)
              }
            }
          } else {
            Some(WindowsLanguageID::FrenchFrance as u16)
          }
        }
        "GU" => {
          Some(WindowsLanguageID::Gujarati as u16)
        }
        "HE" => {
          Some(WindowsLanguageID::Hebrew as u16)
        }
        "HI" => {
          Some(WindowsLanguageID::Hindi as u16)
        }
        "HR" => {
          Some(WindowsLanguageID::Croatian as u16)
        }
        "HU" => {
          Some(WindowsLanguageID::Hungarian as u16)
        }
        "IS" => {
          Some(WindowsLanguageID::Icelandic as u16)
        }
        "ID" => {
          Some(WindowsLanguageID::Indonesian as u16)
        }
        "IT" => {
          if let Some(extend_language) = extended_language {
            match extend_language {
              "CH" => {
                Some(WindowsLanguageID::ItalianSwitzerland as u16)
              }
              "IT" => {
                Some(WindowsLanguageID::ItalianItaly as u16)
              }
              _ => {
                Some(WindowsLanguageID::ItalianItaly as u16)
              }
            }
          } else {
            Some(WindowsLanguageID::ItalianItaly as u16)
          }
        }
        "JA" => {
          Some(WindowsLanguageID::Japanese as u16)
        }
        "KN" => {
          Some(WindowsLanguageID::Kannada as u16)
        }
        "KO" => {
          Some(WindowsLanguageID::Korean as u16)
        }
        "LA" => {
          Some(WindowsLanguageID::Latin as u16)
        }
        "LV" => {
          Some(WindowsLanguageID::Latvian as u16)
        }
        "LT" => {
          Some(WindowsLanguageID::Lithuanian as u16)
        }
        "MS" => {
          Some(WindowsLanguageID::MalayMalaysia as u16)
        }
        "ML" => {
          Some(WindowsLanguageID::Malayalam as u16)
        }
        "MT" => {
          Some(WindowsLanguageID::Maltese as u16)
        }
        "MR" => {
          Some(WindowsLanguageID::Marathi as u16)
        }
        "MN" => {
          Some(WindowsLanguageID::MongolianCyrillic as u16)
        }
        "NE" => {
          Some(WindowsLanguageID::Nepali as u16)
        }
        "NO" => {
          Some(WindowsLanguageID::NorwegianBokmål as u16)
        }
        "OR" => {
          Some(WindowsLanguageID::Oriya as u16)
        }
        "PL" => {
          Some(WindowsLanguageID::Polish as u16)
        }
        "PT" => {
          if let Some(extend_language) = extended_language {
            match extend_language {
              "BR" => {
                Some(WindowsLanguageID::PortugueseBrazil as u16)
              }
              "PT" => {
                Some(WindowsLanguageID::PortuguesePortugal as u16)
              }
              _ => {
                Some(WindowsLanguageID::PortuguesePortugal as u16)
              }
            }
          } else {
            Some(WindowsLanguageID::PortuguesePortugal as u16)
          }
        }
        "PA" => {
          Some(WindowsLanguageID::Punjabi as u16)
        }
        "RM" => {
          Some(WindowsLanguageID::RhaetoRomanic as u16)
        }
        "RO" => {
          Some(WindowsLanguageID::Romanian as u16)
        }
        "RU" => {
          Some(WindowsLanguageID::Russian as u16)
        }
        "SA" => {
          Some(WindowsLanguageID::Sanskrit as u16)
        }
        "SR" => {
          Some(WindowsLanguageID::SerbianCyrillic as u16)
        }
        "SK" => {
          Some(WindowsLanguageID::Slovak as u16)
        }
        "SL" => {
          Some(WindowsLanguageID::Slovenian as u16)
        }
        "SW" => {
          Some(WindowsLanguageID::Kiswahili as u16)
        }
        "SV" => {
          if let Some(extend_language) = extended_language {
            match extend_language {
              "FI" => {
                Some(WindowsLanguageID::SwedishFinland as u16)
              }
              "SE" => {
                Some(WindowsLanguageID::SwedishSweden as u16)
              }
              _ => {
                Some(WindowsLanguageID::SwedishSweden as u16)
              }
            }
          } else {
            Some(WindowsLanguageID::SwedishSweden as u16)
          }
        }
        "TA" => {
          Some(WindowsLanguageID::Tamil as u16)
        }
        "TT" => {
          Some(WindowsLanguageID::Tatar as u16)
        }
        "TE" => {
          Some(WindowsLanguageID::Telugu as u16)
        }
        "TH" => {
          Some(WindowsLanguageID::Thai as u16)
        }
        "BO" => {
          Some(WindowsLanguageID::TibetanPRC as u16)
        }
        "TR" => {
          Some(WindowsLanguageID::Turkish as u16)
        }
        "UK" => {
          Some(WindowsLanguageID::Ukrainian as u16)
        }
        "UR" => {
          Some(WindowsLanguageID::Urdu as u16)
        }
        "UZ" => {
          Some(WindowsLanguageID::UzbekCyrillic as u16)
        }
        "VI" => {
          Some(WindowsLanguageID::Vietnamese as u16)
        }
        "CY" => {
          Some(WindowsLanguageID::Welsh as u16)
        }
        "XH" => {
          Some(default as u16)
        }
        "YO" => {
          Some(WindowsLanguageID::Yoruba as u16)
        }
        "ZA" => {
          Some(default as u16)
        }
        _ => {
          Some(default as u16)
        }
      }
    },
    _ => None
  }
}
