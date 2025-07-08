use crate::types::Language;

pub struct TypingTexts;

impl TypingTexts {
    pub fn get_sample_texts(language: &Language) -> Vec<String> {
        match language {
            Language::Japanese => vec![
                "konnichiwa sekai".to_string(),
                "watashi no namae wa tanaka desu".to_string(),
                "kyou wa ii tenki desu ne".to_string(),
                "nihongo wo benkyou shite imasu".to_string(),
                "arigato gozaimasu".to_string(),
                "sumimasen chotto matte kudasai".to_string(),
                "eigo to nihongo wo hanashimasu".to_string(),
                "ashita wa yasumi desu".to_string(),
                "takusan tabete takusan nete takusan asobu".to_string(),
                "pasokon de shigoto wo shite imasu".to_string(),
            ],
            Language::English => vec![
                "hello world".to_string(),
                "the quick brown fox jumps over the lazy dog".to_string(),
                "programming is fun and challenging".to_string(),
                "practice makes perfect".to_string(),
                "typing speed and accuracy are important".to_string(),
                "learning new skills takes time and patience".to_string(),
                "technology changes rapidly in modern times".to_string(),
                "communication skills are valuable in any career".to_string(),
                "reading books expands your knowledge and vocabulary".to_string(),
                "staying healthy requires exercise and good nutrition".to_string(),
            ],
        }
    }

    pub fn get_beginner_texts(language: &Language) -> Vec<String> {
        match language {
            Language::Japanese => vec![
                "aaa".to_string(),
                "abc".to_string(),
                "aiueo".to_string(),
                "hello".to_string(),
                "arigatou".to_string(),
            ],
            Language::English => vec![
                "aaa".to_string(),
                "abc".to_string(),
                "cat".to_string(),
                "dog".to_string(),
                "hello".to_string(),
            ],
        }
    }

    pub fn get_intermediate_texts(language: &Language) -> Vec<String> {
        match language {
            Language::Japanese => vec![
                "ohayou gozaimasu".to_string(),
                "genki desu ka".to_string(),
                "sumimasen".to_string(),
                "douzo yoroshiku".to_string(),
                "mata ashita".to_string(),
            ],
            Language::English => vec![
                "good morning".to_string(),
                "how are you".to_string(),
                "nice to meet you".to_string(),
                "see you tomorrow".to_string(),
                "have a great day".to_string(),
            ],
        }
    }

    pub fn get_advanced_texts(language: &Language) -> Vec<String> {
        match language {
            Language::Japanese => vec![
                "watakushi wa nihon no bunka ni kyoumi ga arimasu".to_string(),
                "gijutsu no shinpo wa ningen no seikatsu wo kaete imasu".to_string(),
                "kyouiku wa mirai no shakai wo tsukuru tame ni taisetsu desu".to_string(),
                "kokusai kankei wa fukuzatsu de muzukashii mondai desu".to_string(),
                "kankyou mondai wa chikyuu zentai no kadai to natte imasu".to_string(),
            ],
            Language::English => vec![
                "artificial intelligence is transforming various industries".to_string(),
                "sustainable development requires global cooperation and commitment".to_string(),
                "digital transformation has accelerated due to recent events".to_string(),
                "climate change poses significant challenges for future generations".to_string(),
                "effective communication bridges cultural and linguistic differences".to_string(),
            ],
        }
    }

    pub fn get_random_text(language: &Language) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let texts = Self::get_sample_texts(language);
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        
        let index = (hasher.finish() as usize) % texts.len();
        texts[index].clone()
    }
}