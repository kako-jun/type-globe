# Part 2 / 3: index 228..455 (228 entries)
# A = actual (existing ja_typings) is wrong; pykakasi-derived expected is correct
# B = both readings are valid kanji-yomi variants
# C = pykakasi mis-reads; actual is correct (proper noun / title / coined term / etc.)
DECISIONS_PART2 = {
    # 228 q00336 "文明崩壊後の地球" hira ぶんめいほうかいのち... mis-tokenizes 後 as 'のち';
    # actual bunmeihokaigonochikyuu also weird ('ご' for 後 is fine but trailing 'nochi' artifact).
    # actual is closer to correct (崩壊後=ほうかいご), but contains 'nochi' extra -> still wrong overall.
    # pykakasi expected also wrong. Judge by actual: 'bunmeihokaigonochikyuu' has extra 'nochi'. -> C
    ("q00336", 3): ("C", "崩壊後=ほうかいご is correct but actual contains stray 'nochi'; pykakasi also misreads"),
    # 229 q00338 根里俊 -> ねさとしゅん vs konsatoshi: both fictional/invented name; pykakasi gives ne-sato-shun
    # which is plausible kanji-by-kanji. actual 'konsatoshi' looks like a different person name guess. -> C
    ("q00338", 2): ("C", "Invented/fictional name; pykakasi reading reasonable, actual is a different name guess"),
    # 230 q00339 鋼の錬金術師 hira こうのれんきんじゅつし; correct is はがねの -> actual haganenorenkinjutsushi correct -> C
    ("q00339", 1): ("C", "Title '鋼の錬金術師' = はがねの〜; actual correct, pykakasi mis-read 鋼 as こう"),
    # 231 q00340 緑谷出久 (My Hero Academia) みどりや・いずく; hira みどりたにしゅつきゅう wrong -> actual correct -> C
    ("q00340", 0): ("C", "Character name 緑谷出久=みどりやいずく; actual correct, pykakasi wrong"),
    # 232 q00340 爆豪勝己 ばくごう・かつき; actual bakugokatsuki correct vs katsumi -> C
    ("q00340", 1): ("C", "Character name 爆豪勝己=ばくごうかつき; actual correct"),
    # 233 q00349 U.A.高校 (ゆうえい高校); actual yueikoko correct -> C
    ("q00349", 0): ("C", "U.A.高校 reads ゆうえい; actual correct, pykakasi literal"),
    # 234 q00349 式鉄高校 hira しきてつ vs actual shiketsuko (雄英? no 式鉄). invented name; pykakasi plausible
    # actual 'shiketsuko' doesn't match either; could be 'しきてつ' valid -> A (actual is wrong reading)
    ("q00349", 1): ("A", "式鉄=しきてつ natural; actual 'shiketsuko' drops a syllable"),
    # 235 q00349 結仏学園 けつぶつがくえん standard; hira 'けつぶつがくその' (園=その) is wrong reading
    # actual ketsubutsugakuen is correct -> C
    ("q00349", 2): ("C", "学園=がくえん standard; actual correct, pykakasi misreads 園 as その"),
    # 236 q00354 千と千尋の神隠し せん と ちひろ; hira せんとせんじん wrong -> actual correct -> C
    ("q00354", 1): ("C", "Title reads せんとちひろ; actual correct"),
    # 237 q00356 大人男性向け otonadansei vs actual otonanodansei: 大人男性 could read either as 'おとなだんせい'
    # or 'おとなのだんせい'; nuance. Both valid -> B
    ("q00356", 2): ("B", "大人男性 reading vary: 'おとなだんせい' vs 'おとなのだんせい'"),
    # 238 q00357 若男性 (coined). hira わかおせい wrong; actual wakaidanseimuke = 若い男性向け. -> C (actual closer)
    ("q00357", 1): ("C", "若男性 coined; actual interprets as 若い男性 (natural); pykakasi reads incorrectly"),
    # 239 q00357 女子主人公 reading: joshishujinko (じょし) vs joshujinko ; actual drops 'shi' -> A
    ("q00357", 3): ("A", "女子=じょし; actual loses 'shi' syllable"),
    # 240 q00359 公式ボイスアクト: actual koshikiseiyuuengi maps to 'public 声優演技' - different word! -> A
    ("q00359", 1): ("A", "ボイスアクト transliteration; actual replaces with 声優演技 - mismatched text"),
    # 241 q00361 鋼の錬金術師 same as 230 -> C
    ("q00361", 1): ("C", "Title 鋼の錬金術師=はがねの〜; actual correct"),
    # 242 q00361 神奈満 (ノラガミ?). hira かんなまん vs actual noragami - actual replaces with title -> C
    ("q00361", 2): ("C", "神奈満 (kana-man) read as noragami in actual - intentional title mapping"),
    # 243 q00361 青の祓魔師 あおのエクソシスト (official); actual aonoekusoshisuto correct -> C
    ("q00361", 3): ("C", "Title 青の祓魔師=あおのエクソシスト official; actual correct"),
    # 244 q00362 人に渡せる個性 人=にん vs ひと: 'hito ni wataseru' more natural -> B
    ("q00362", 0): ("B", "人 reads にん or ひと; 'ひとに' more natural for context"),
    # 245 q00363 甘藷と稲妻 should be 甘々と稲妻 (amaama)? actual amaamatoinazuma matches Amaama title -> C
    ("q00363", 2): ("C", "Title 甘々と稲妻=あまあまといなずま; actual correct"),
    # 246 q00365 葡萄流派 ぶどうりゅうは vs actual budonoryuuha (の inserted): both possible -> B
    ("q00365", 2): ("B", "葡萄流派 reads ぶどうりゅうは or ぶどうのりゅうは"),
    # 247 q00365 錬金薬 れんきんやく standard; pykakasi gives renkinkusuri (薬=くすり). actual renkinyaku correct -> C
    ("q00365", 3): ("C", "錬金薬=れんきんやく as compound; actual correct"),
    # 248 q00368 高畑清 (Studio Ghibli) 高畑勲(いさお); 清=きよし vs 勲=いさお - text differs from official name
    # actual takahataisao reads as if the ja were 高畑勲. -> C (intentional override)
    ("q00368", 1): ("C", "高畑清: pykakasi reads きよし literal; actual reads as 勲(いさお) intentional"),
    # 249 q00368 根里俊 -> same pattern as 229 -> C
    ("q00368", 2): ("C", "Invented name; actual is a guessed mapping"),
    # 250 q00368 押井修 -> 押井守(まもる); actual oshiimamoru intentional override -> C
    ("q00368", 3): ("C", "押井修 pykakasi=おさむ literal; actual=まもる intentional mapping to 押井守"),
    # 251 q00370 鬼滅の刃 きめつのやいば; hira おにほろのは wrong; actual kimetsunoyaiba correct -> C
    ("q00370", 0): ("C", "Title 鬼滅の刃=きめつのやいば; actual correct"),
    # 252 q00371 螺旋眼 -> 螺旋丸/写輪眼? hira らせんめ; actual rasengan (螺旋丸=らせんがん) -> C
    ("q00371", 1): ("C", "螺旋眼 read as 螺旋丸/眼=がん intentional title mapping"),
    # 253 q00373 人を殺す: にんをころす vs ひとをころす; ひと is natural -> A
    ("q00373", 0): ("A", "人を殺す natural reading is ひとをころす; actual correct"),
    # 254 q00375 戦う技 たたかう・わざ; actual butonowaza (武道?) doesn't match -> A
    ("q00375", 3): ("A", "戦う技=たたかうわざ; actual reads as different word 武道の技"),
    # 255 q00376 神束 (Noragami?). actual noragami same pattern -> C
    ("q00376", 2): ("C", "神束 mapped to noragami in actual - title mapping"),
    # 256 q00377 巨大な人間型生物 生物=せいぶつ vs いきもの: both valid -> B
    ("q00377", 0): ("B", "生物 reads せいぶつ or いきもの; both valid"),
    # 257 q00380 same as 228 -> C
    ("q00380", 3): ("C", "崩壊後 reading; actual contains stray 'nochi'; pykakasi misreads as well"),
    # 258 q00381 岸本正史 岸本斉史(まさし)official actual kishimotomasashi -> C (intentional)
    ("q00381", 1): ("C", "岸本正史 official=斉史(まさし); actual=masashi intentional"),
    # 259 q00381 久保帯人 = くぼ・たいと official; hira くぼおびにん wrong -> actual correct -> C
    ("q00381", 2): ("C", "久保帯人=くぼたいと; actual correct"),
    # 260 q00381 鳥山明 = とりやま・あきら official; hira mei wrong -> actual correct -> C
    ("q00381", 3): ("C", "鳥山明=とりやまあきら; actual correct"),
    # 261 q00384 誘刻 -> 憂国(ゆうこく)? title 'ユウコクのモリアーティ'. actual yuukoku closer -> C
    ("q00384", 2): ("C", "Title 憂国のモリアーティ=ゆうこく; actual approximates"),
    # 262 q00386 神束=noragami pattern -> C
    ("q00386", 1): ("C", "神束 mapped to noragami"),
    # 263 q00386 青の祓魔師 same -> C
    ("q00386", 2): ("C", "Title 青の祓魔師=エクソシスト official"),
    # 264 q00390 13話 actual reads 'wa' (話=わ counter), pykakasi reads はなし. counter is わ -> A
    ("q00390", 0): ("A", "13話 counter reads 13わ; actual correct, pykakasi misreads"),
    # 265 q00390 配信シーズンフォーマット -> actual haishinkisetsukeishiki = 配信季節形式 - different text -> A?
    # actual translates シーズン->季節 and フォーマット->形式: mismatched transliteration to translation -> A
    ("q00390", 3): ("A", "Actual translates katakana to kanji (different text from ja)"),
    # 266 q00391 鬼滅の刃 -> C
    ("q00391", 1): ("C", "鬼滅の刃 title; actual correct"),
    # 267 q00392 物語=ものがたり vs 話=はなし: actual changes 物語->話. mismatch -> A
    ("q00392", 0): ("A", "Actual rewrites 物語 to 話/品質to品 - text mismatched"),
    # 268 q00392 暴力的 -> hakyokuteki (破壊的?) actual replaces. mismatch -> A
    ("q00392", 3): ("A", "Actual replaces 暴力 with 破壊 - text mismatched"),
    # 269 q00394 岸本正史 = masashi intentional -> C
    ("q00394", 1): ("C", "岸本斉史 official; actual masashi intentional"),
    # 270 q00394 鳥山明=あきら -> C
    ("q00394", 2): ("C", "鳥山明=あきら; actual correct"),
    # 271 q00398 新世紀エヴァンゲリオン エヴァ vs エンガ: pykakasi mis-tokens ヴァ -> A
    ("q00398", 3): ("A", "エヴァンゲリオン standard; actual 'engaerion' mis-types ヴァ as エ"),
    # 272 q00399 日本 にっぽん vs にほん -> B
    ("q00399", 2): ("B", "日本=にっぽん/にほん both valid"),
    # 273 q00399 same 崩壊後 pattern -> C
    ("q00399", 3): ("C", "崩壊後 reading nuance"),
    # 274 q00404 幽遊白書 ゆうゆうはくしょ vs yuyuhakusho: actual elides long う ->
    # ja_reading uses ゆうゆう (correct), expected yuuyuuhakusho. actual yuyuhakusho is shortened.
    # Hepburn 'yuuyuu' vs 'yuyu' - long vowel collapse acceptable but project disallows wapuro collapses
    # within ja_typings? Per skill rules, long vowel non-collapse and collapse both allowed.
    # So actual yuyuhakusho corresponds to collapsed long vowel -> valid alternate -> B
    ("q00404", 1): ("B", "幽遊白書 long vowel collapse vs non-collapse; both allowed"),
    # 275 q00405 エドワードの自動メール肢 -> actual edowadonogisokushishi (義足肢) - text mismatched -> A
    ("q00405", 0): ("A", "Actual rewrites メール肢 as 義足肢 - text mismatched"),
    # 276 q00406 転生したら hira てんしょう vs てんせい; correct is てんせい -> A
    ("q00406", 2): ("A", "転生=てんせい standard; actual correct"),
    # 277 q00407 日本=にっぽん/にほん -> B
    ("q00407", 1): ("B", "日本 reading nuance"),
    # 278 q00411 六眼 (Jujutsu Kaisen) = ろくがん official; hira ろくめ wrong -> actual correct -> C
    ("q00411", 1): ("C", "六眼=ろくがん official; actual correct"),
    # 279 q00411 反転呪術式 - actual drops 'ju' (jujutsu->jutsu)? actual hantenjutsushiki vs expected hantenjujutsushiki
    # Term 反転術式 (はんてんじゅつしき) is correct JJK term; ja has 呪術式 extra 呪 -> ja text has extra char
    # actual treats as 反転術式 (correct title term) -> intentional -> C
    ("q00411", 2): ("C", "Official JJK term 反転術式=はんてんじゅつしき; actual correct (ja has stray 呪)"),
    # 280 q00411 領位展開 -> 領域展開(りょういき); intentional -> C
    ("q00411", 3): ("C", "Official 領域展開=りょういきてんかい; actual correct"),
    # 281 q00412 心理スリラー hira しんりすりらー vs actual shinrisasupensu (サスペンス) - text mismatch -> A
    ("q00412", 0): ("A", "Actual rewrites スリラー as サスペンス - text mismatched"),
    # 282 q00413 任天堂64 nintendo64 vs nintendorokujuuyon; '64' often read 'ろくじゅうよん' literally
    # Both readings exist - the brand is 'にんてんどうろくよん' (rokuyon). Both valid -> B
    ("q00413", 0): ("B", "64 reads as digits or numeric reading; both valid"),
    # 283 q00424 same 崩壊後 -> C
    ("q00424", 3): ("C", "崩壊後 reading nuance"),
    # 284 q00427 消す けす vs きえす: actual kiesu (消える?) -> A (actual extra 'i')
    ("q00427", 0): ("A", "消す=けす; actual adds spurious 'i'"),
    # 285 q00427 敵を倒す hira かなをたおす wrong (敵=かな?? no, てき). actual tekiotaosu correct -> A
    ("q00427", 1): ("A", "敵=てき standard; actual correct, pykakasi misreads"),
    # 286 q00438 一次純 hira いちじじゅん vs ichijun (loses 'ji'): both possible compound readings -> B
    ("q00438", 3): ("B", "一次純 reading: いちじじゅん or いちじゅん; both possible"),
    # 287 q00439 チェックメイト hira チェックメイト vs actual chekkumeto (long vowel collapsed) -> B
    ("q00439", 0): ("B", "Long vowel collapse allowed"),
    # 288 q00439 全手つきょうする -> actual zentehatsukyosuru (全手発？): text mismatch -> A
    ("q00439", 1): ("A", "Actual reads 'tehatsu' instead of 'tetsukyo'; misreading"),
    # 289 q00449 ダウンロード可能 hira だうんろーどかのう vs actual ...kanona (extra 'na'): -> A
    ("q00449", 0): ("A", "Actual has spurious 'na' insertion"),
    # 290 q00455 アリーナ vs arena (long vowel collapse) -> B
    ("q00455", 0): ("B", "Long vowel arena/arīna both allowed"),
    # 291 q00455 バトルアサインメント vs actual batorukadai (課題) - text mismatch -> A
    ("q00455", 3): ("A", "Actual rewrites katakana as 課題 - text mismatched"),
    # 292 q00457 主題 -> actual himei? 'sento no himei' (悲鳴?) - completely different -> A
    ("q00457", 2): ("A", "Actual swaps 主題 for 悲鳴 - text mismatched"),
    # 293 q00459 終わった おわった vs actual gawatta (loses 'o'): typo -> A
    ("q00459", 2): ("A", "終わった=おわった; actual drops 'o' producing 'gawatta'"),
    # 294 q00459 ローディング ろーでぃんぐ vs rodogamen (drops 'in'): -> A
    ("q00459", 3): ("A", "Actual drops 'in' from ローディング"),
    # 295 q00460 多人 (たにん vs たじん) - coined; both possible -> B
    ("q00460", 0): ("B", "多人 coined reading nuance; both possible"),
    # 296 q00460 same -> B
    ("q00460", 1): ("B", "多人 coined reading nuance"),
    # 297 q00463 hira 'びょうそく' vs actual 'byoshokunisokuji' weird extra -> A
    ("q00463", 0): ("A", "Actual adds spurious 'nisokuji'"),
    # 298 q00464 高レイテンシー: 高=たか vs こう; こう more natural for compound with レイテンシー -> A (actual correct)
    ("q00464", 0): ("A", "高 compound=こう; actual correct (こうれいてんしー)"),
    # 299 q00464 サーバーメンテナンスウィンドウ vs actual omits 'window' -> A
    ("q00464", 2): ("A", "Actual drops 'windou' segment"),
    # 300 q00467 列 つら vs れつ: actual retsu standard -> A
    ("q00467", 2): ("A", "列=れつ standard; actual correct"),
    # 301 q00468 対 ついvs たい: 'たい' standard for 'vs.' -> A
    ("q00468", 0): ("A", "対=たい (vs.) standard; actual correct"),
    # 302 q00468 same 対 -> A
    ("q00468", 1): ("A", "対=たい standard"),
    # 303 q00468 same 対 -> A
    ("q00468", 2): ("A", "対=たい standard"),
    # 304 q00468 自動体づくり -> actual jidotaitezukuri vs jidokaradazukuri: 体=からだ vs たい; ambiguous compound -> B
    ("q00468", 3): ("B", "体=たい/からだ both valid in compound"),
    # 305 q00469 対 -> A
    ("q00469", 0): ("A", "対=たい standard"),
    # 306 q00469 hira 対=たい already; actual 'pureiyateki' drops 対 -> A (existing actual incomplete)
    ("q00469", 1): ("A", "Actual drops 対 syllable"),
    # 307 q00469 対 -> A
    ("q00469", 2): ("A", "対=たい standard"),
    # 308 q00469 体験値 actual taikenchi vs taikenatai: 値=ち vs あたい; ち standard in 体験値 -> A
    ("q00469", 3): ("A", "値 in 体験値=ち (compound); actual correct"),
    # 309 q00470 ステータス vs sutatsu: long vowel collapse -> B
    ("q00470", 0): ("B", "Long vowel collapse for ステータス allowed"),
    # 310 q00471 文字 もじ vs キャラ: actual 'kyara' rewrites text -> A
    ("q00471", 2): ("A", "Actual rewrites 文字 as キャラ - text mismatched"),
    # 311 q00475 ダーク vs くらい (暗い): actual 'kurai' rewrites -> A
    ("q00475", 0): ("A", "Actual rewrites ダーク as 暗い - text mismatched"),
    # 312 q00475 日本=にっぽん/にほん -> B
    ("q00475", 2): ("B", "日本 reading nuance"),
    # 313 q00475 崩壊後 -> C
    ("q00475", 3): ("C", "崩壊後 reading nuance"),
    # 314 q00477 木 hira き vs actual moku: 木=き common, もく compound; both valid -> B
    ("q00477", 0): ("B", "木=き/もく both valid"),
    # 315 q00478 報酬 ほうしゅう vs hosho: long vowel collapse -> B
    ("q00478", 0): ("B", "報酬 long vowel collapse"),
    # 316 q00479 フレーム数 hira かず vs actual suu: both valid -> B
    ("q00479", 0): ("B", "数=かず/すう both valid"),
    # 317 q00479 滑らかさ; actual drops 'no'; minor -> B (both readings exist with or without の)
    ("q00479", 3): ("B", "アニメーションのなめらかさ optional の"),
    # 318 q00480 ディスプレイ vs ti-supurei: actual 'tisupurei' renders ディ as ティ (non-collapse vs wapuro);
    # 'di' standard but 'ti' would be different katakana. ディ=di. Actual incorrectly uses 'ti' -> A
    ("q00480", 0): ("A", "ディ=di; actual uses 'ti' instead - wrong"),
    # 319 q00480 設定 vs のせってい: の addition: both okay -> B
    ("q00480", 3): ("B", "Optional の particle"),
    # 320 q00481 最初 さいしょ vs さいしょき (最初期): actual adds 'ki' -> A
    ("q00481", 0): ("A", "最初=さいしょ; actual adds spurious 'ki'"),
    # 321 q00482 弾がん密度 -> actual bisshiri (びっしり) rewrites -> A
    ("q00482", 0): ("A", "Actual rewrites 弾幕密度 as びっしり - text mismatched"),
    # 322 q00482 シューティング vs 射撃(shageki): actual rewrites -> A
    ("q00482", 2): ("A", "Actual rewrites シューティング as 射撃 - text mismatched"),
    # 323 q00485 時間制限 vs actual jikanseinobatorumodo (時間制?): actual drops 'gen' -> A
    ("q00485", 3): ("A", "Actual drops 限 syllable"),
    # 324 q00488 非依存 vs actual gyotoku (御徳?): nonsensical -> A
    ("q00488", 3): ("A", "Actual reads 非依存 as 'gyotoku' - wrong"),
    # 325 q00489 ゲームプラットフォーム vs actual 'gemuki' (ゲーム機?): rewrites -> A
    ("q00489", 1): ("A", "Actual rewrites ゲームプラットフォーム; mismatched"),
    # 326 q00489 オンラインプラットフォーム vs purattofamu (long vowel collapse): both acceptable -> B
    ("q00489", 3): ("B", "Long vowel collapse for プラットフォーム"),
    # 327 q00490 探索を end -> actual drops trailing syllables -> A
    ("q00490", 0): ("A", "Actual truncates ending '探索 を'"),
    # 328 q00490 探査 たんさ vs たんさく: pykakasi reads たんさ correctly; actual uchuutansakugemu adds 'ku' -> A
    ("q00490", 1): ("A", "宇宙探査=うちゅうたんさ; actual reads as 探索 with extra 'ku'"),
    # 329 q00490 キャスティーヴァニア vs カストルヴァニア: both transliteration variants -> B
    ("q00490", 2): ("B", "Castlevania transliteration variants"),
    # 330 q00491 八世代 はっせい vs はちせ: counter readings vary -> B
    ("q00491", 1): ("B", "八世代=はっせいだい/はちせだい both possible"),
    # 331 q00491 十世代 じゅうせ vs じっせ -> B
    ("q00491", 2): ("B", "十世代 reading variation"),
    # 332 q00491 七世代 しちせ vs ななせ -> B
    ("q00491", 3): ("B", "七世代=しち/なな both valid"),
    # 333 q00492 大手 hira おおて; actual omits 'おおて' -> A
    ("q00492", 0): ("A", "Actual drops 大手 prefix"),
    # 334 q00492 未リリース hira ひつじ wrong; actual 'miririsu' correct (未=み) -> C
    ("q00492", 3): ("C", "未=み standard; actual correct, pykakasi misreads 未 as ひつじ"),
    # 335 q00493 プラットフォーム vs プラットフォーム (long vowel collapse) -> B
    ("q00493", 0): ("B", "Long vowel collapse"),
    # 336 q00493 エンジン -> actual omits 'kaihatsu' (開発) -> A
    ("q00493", 1): ("A", "Actual drops 開発 segment"),
    # 337-340 q00494 日本=にっぽん/にほん -> B
    ("q00494", 0): ("B", "日本 reading nuance"),
    ("q00494", 1): ("B", "日本 reading nuance"),
    ("q00494", 2): ("B", "日本 reading nuance"),
    ("q00494", 3): ("B", "日本 reading nuance"),
    # 341-343 q00495 作者名: official readings -> C
    ("q00495", 0): ("C", "鳥山明=あきら official"),
    ("q00495", 2): ("C", "岸本斉史 official intentional"),
    ("q00495", 3): ("C", "久保帯人=たいと official"),
    # 344 q00496 鬼滅の刃 -> C
    ("q00496", 3): ("C", "鬼滅の刃 title official"),
    # 345 q00498 版 はん vs ばん: 'ばん' standard -> A (actual correct)
    ("q00498", 2): ("A", "版=ばん standard; actual correct"),
    # 346 q00502 読み方 よみかた vs よみがた: rendaku optional -> B
    ("q00502", 0): ("B", "読み方=よみかた/よみがた rendaku optional"),
    # 347 q00502 音響効果 -> actual onsayo (音作用?): rewrites -> A
    ("q00502", 1): ("A", "Actual rewrites 音響効果 - mismatched"),
    # 348 q00502 名 めい vs 名前 namae: actual extends to なまえ -> A
    ("q00502", 3): ("A", "Actual rewrites 名 as 名前 - text mismatched"),
    # 349 q00503 諫山創 = いさやま・はじめ official; hira そう wrong -> actual correct -> C
    ("q00503", 1): ("C", "諫山創=はじめ official; actual correct"),
    # 350 q00504 敵=かな (hira wrong) vs てき; actual correct -> A
    ("q00504", 0): ("A", "敵=てき standard; actual correct, pykakasi wrong"),
    # 351 q00504 食べ物で戦う料理人 actual drops 'tatakau' -> A
    ("q00504", 2): ("A", "Actual drops 戦う syllables"),
    # 352 q00505 四つ葉 hira よつは vs よつば: rendaku -> B
    ("q00505", 3): ("B", "四つ葉=よつば rendaku; actual correct"),
    # 353 q00509 縦長電子漫画 actual drops 'denshi' -> A
    ("q00509", 0): ("A", "Actual drops 電子 (denshi) segment"),
    # 354 q00509 日本漫画 にっぽん/にほん -> B
    ("q00509", 2): ("B", "日本 reading nuance"),
    # 355-357 q00510 author names -> C
    ("q00510", 0): ("C", "久保帯人=たいと official"),
    ("q00510", 1): ("C", "岸本斉史 intentional"),
    ("q00510", 2): ("C", "鳥山明=あきら official"),
    # 358 q00512 自販 hira じはん vs actual 'jigan' (自顔?): -> A (jihan correct)
    ("q00512", 0): ("A", "自販=じはん standard; actual 'jigan' wrong"),
    # 359 q00512 極蔵 -> actual kyokazumi: invented name; both guess -> C
    ("q00512", 2): ("C", "Invented name; both readings speculative"),
    # 360 q00515 骨喰い hira ほねくい vs actual kokami (骨神?): ほねくい standard -> A (actual wrong)
    ("q00515", 2): ("A", "骨喰い=ほねくい; actual 'kokami' wrong"),
    # 361 q00516 岸本斉史 intentional -> C
    ("q00516", 1): ("C", "岸本斉史 official intentional"),
    # 362 q00516 鳥山明 -> C
    ("q00516", 2): ("C", "鳥山明=あきら official"),
    # 363 q00517 古代日本 にっぽん/にほん -> B
    ("q00517", 1): ("B", "日本 reading nuance"),
    # 364 q00517 中世日本 same -> B
    ("q00517", 2): ("B", "日本 reading nuance"),
    # 365 q00518 写実的 hira しゃじつ vs しじつ: しゃじつ standard; actual drops 'a'? actual shijitsu wrong -> A
    ("q00518", 0): ("A", "写実的=しゃじつてき; actual drops 'ya'"),
    # 366 q00518 面白い おもしろい vs もしろ: actual drops 'i' -> A
    ("q00518", 1): ("A", "面白い=おもしろい; actual missing trailing 'i'"),
    # 367 q00518 アニメ形式 vs animeka (drops syllable?): actual adds 'ka'? animeka extra -> A
    ("q00518", 3): ("A", "Actual inserts spurious 'ka'"),
    # 368 q00519 鋼の錬金術師 -> C
    ("q00519", 0): ("C", "鋼の錬金術師=はがねの〜; actual correct"),
    # 369 q00519 神奈満=noragami -> C
    ("q00519", 1): ("C", "神奈満→noragami title mapping"),
    # 370 q00519 青の祓魔師=エクソシスト -> C
    ("q00519", 2): ("C", "青の祓魔師=エクソシスト official"),
    # 371 q00521 転生 てんしょう vs てんせい: てんせい standard -> A
    ("q00521", 3): ("A", "転生=てんせい; actual correct"),
    # 372 q00524 鋼の錬金術師 -> C
    ("q00524", 0): ("C", "鋼の錬金術師 title"),
    # 373 q00527 版 はん vs ばん: ばん standard -> A
    ("q00527", 2): ("A", "版=ばん standard"),
    # 374 q00527 same 版 -> A
    ("q00527", 3): ("A", "版=ばん standard"),
    # 375 q00530 対作 hira ついさく vs たいさく: たいさく standard -> A
    ("q00530", 2): ("A", "対作=たいさく standard"),
    # 376 q00531 渡辺義弘 watanabeyoshihiro vs togashiyoshihiro: actual rewrites 渡辺 as 冨樫? -> intentional?
    # The author name is likely 冨樫義博(よしひろ); ja has 渡辺義弘 (different person). actual maps to togashi -> C
    ("q00531", 3): ("C", "Actual maps 渡辺義弘 to 冨樫義博 - intentional mapping"),
    # 377 q00538 複雑 ふくざつ vs fukuzatsuna (adds 'na'): natural adjective -> B
    ("q00538", 0): ("B", "複雑 as adjective can have な; both readings okay"),
    # 378 q00538 単純な vs 簡単な: actual rewrites 単純 to 簡単 -> A
    ("q00538", 1): ("A", "Actual rewrites 単純 as 簡単 - text mismatched"),
    # 379-382 q00540 author names -> C
    ("q00540", 0): ("C", "渡辺→冨樫義博 intentional"),
    ("q00540", 1): ("C", "岸本斉史 official"),
    ("q00540", 2): ("C", "久保帯人=たいと official"),
    ("q00540", 3): ("C", "鳥山明=あきら official"),
    # 383 q00541 漫画家 まんがか vs 漫画いえ(まんがいえ): いえ wrong -> A (actual correct)
    ("q00541", 2): ("A", "漫画家=まんがか; actual correct, pykakasi misreads 家"),
    # 384 q00542 幽遊白書 long vowel -> B
    ("q00542", 1): ("B", "幽遊白書 long vowel collapse"),
    # 385 q00543 四コマ しこま vs よんこま: よんこま standard for counter -> A
    ("q00543", 0): ("A", "四コマ=よんこま standard"),
    # 386 q00543 四巻 しかん vs よんかん: よんかん standard -> A
    ("q00543", 2): ("A", "四巻=よんかん standard"),
    # 387 q00545 真島広 mashimako vs hiro: ヒロ official (真島ヒロ) intentional -> C
    ("q00545", 0): ("C", "真島ヒロ official; actual hiro intentional"),
    # 388 q00545 岸本斉史 intentional -> C
    ("q00545", 2): ("C", "岸本斉史 official intentional"),
    # 389 q00545 東城義弘 -> 冨樫義博 intentional -> C
    ("q00545", 3): ("C", "東城→冨樫 intentional mapping"),
    # 390 q00547 成人男性 のための テーマ -> actual adds extra のとなのテーマ; -> A (actual contains stray segment)
    ("q00547", 0): ("A", "Actual contains spurious 'notonano' insertion"),
    # 391 q00547 恋愛話 れんあいばなし vs れんあいはなし: rendaku -> B
    ("q00547", 1): ("B", "恋愛話 はなし/ばなし rendaku"),
    # 392 q00547 アークス vs 編(へん): actual rewrites アークス as 編 -> A
    ("q00547", 3): ("A", "Actual rewrites アークス as 編 - text mismatched"),
    # 393 q00548 独特な vs 独特の: particle nuance -> B
    ("q00548", 0): ("B", "独特な/独特の both valid"),
    # 394 q00553 山 やま vs さん: 'エベレスト山' = エベレストさん standard -> A
    ("q00553", 0): ("A", "山 in 'エベレスト山' = さん standard"),
    # 395 q00556 インド洋 いんどよう vs いんどひろし: 洋=よう standard -> A
    ("q00556", 2): ("A", "洋=よう (in 大洋); actual correct"),
    # 396 q00556 南極海 なんきょくかい vs なんきょくうみ: 海=かい standard in 南極海 -> A
    ("q00556", 3): ("A", "海 in compound=かい standard"),
    # 397 q00562 七 しち vs なな: both valid for 7 -> B
    ("q00562", 0): ("B", "七=しち/なな both valid"),
    # 398 q00568 釜山 ふざん vs プサン (pusan): Korean city; プサン (Pusan/Busan) standard -> A
    ("q00568", 1): ("A", "釜山=プサン (Korean place name) standard; actual correct"),
    # 399 q00568 仁川 にかわ vs インチョン: Korean place name; インチョン standard -> A
    ("q00568", 2): ("A", "仁川=インチョン standard Korean; actual correct"),
    # 400 q00568 大邱 たいきゅう vs テグ: Korean place; テグ standard -> A
    ("q00568", 3): ("A", "大邱=テグ standard Korean"),
    # 401 q00574 カスピ海 海=かい (in 'X海' standard) -> A
    ("q00574", 3): ("A", "海 in カスピ海=かい standard"),
    # 402 q00577 バイカル湖 みずうみ vs こ: 湖 in '〜湖' =こ standard -> A
    ("q00577", 0): ("A", "湖 in proper noun=こ standard"),
    # 403 q00577 カスピ海 -> A
    ("q00577", 2): ("A", "海=かい standard"),
    # 404 q00579 インド洋 -> A
    ("q00579", 2): ("A", "洋=よう standard"),
    # 405 q00579 北極海 海=かい -> A
    ("q00579", 3): ("A", "海=かい standard"),
    # 406 q00581 日本 -> B
    ("q00581", 0): ("B", "日本 reading nuance"),
    # 407 q00593 日本 -> B
    ("q00593", 3): ("B", "日本 reading nuance"),
    # 408 q00607 秦始皇帝 hira しんしこうてい vs しこうてい: 'しこうてい' common abbreviated -> B
    ("q00607", 3): ("B", "始皇帝 readable as しこうてい (omits 秦); both possible"),
    # 409-412 q00608 古代...人: 人=にん vs じん: じん standard for nationality -> A
    ("q00608", 0): ("A", "人=じん in nationality compound"),
    ("q00608", 1): ("A", "人=じん in nationality compound"),
    ("q00608", 2): ("A", "人=じん in nationality compound"),
    ("q00608", 3): ("A", "人=じん in nationality compound"),
    # 413 q00610 軍指導者 ぐん vs ぐんじ: actual rewrites 指導者→軍事指導者 changing text -> A
    ("q00610", 0): ("A", "Actual inserts 'ji' (軍事) - text mismatched"),
    # 414 q00612 ソ連 vs ソ: actual truncates -> A (actual has 'sono' nonsense)
    ("q00612", 0): ("A", "Actual truncates dramatically (only 'amerikatosono')"),
    # 415 q00612 寒冷 かんれい vs さむい: actual rewrites as 寒い -> A
    ("q00612", 1): ("A", "Actual rewrites 寒冷 as 寒い - text mismatched"),
    # 416 q00612 武器なき vs 武器なし: なき=poetic, なし=plain; text says なき -> A (actual uses なし)
    ("q00612", 3): ("A", "ja text has なき; actual reads なし - mismatched form"),
    # 417 q00613 7世 hira 7よ vs 7せい: せい for monarch numbering -> A
    ("q00613", 0): ("A", "世 in royal numbering=せい standard"),
    # 418 q00613 2世 -> A
    ("q00613", 1): ("A", "世 in royal numbering=せい"),
    # 419 q00619 英国税 hira えいこくぜい vs actual eikokukazei (英国課税?): actual inserts 'ka' -> A (actual wrong)
    ("q00619", 0): ("A", "Actual inserts spurious 'ka'"),
    # 420 q00619 お祝い vs 祭典: actual rewrites おいわい as さいてん -> A
    ("q00619", 3): ("A", "Actual rewrites お祝い as 祭典 - text mismatched"),
    # 421 q00621 路 みち vs ろ: 貿易路=ぼうえきろ standard compound -> A
    ("q00621", 0): ("A", "路 in 貿易路=ろ standard"),
    # 422 q00621 -> A same
    ("q00621", 3): ("A", "路=ろ standard in compound"),
    # 423 q00622 ギリシャ vs ギリシア: katakana variant -> B
    ("q00622", 1): ("B", "ギリシャ/ギリシア variant"),
    # 424 q00622 ペルシャ vs ペルシア -> B
    ("q00622", 3): ("B", "ペルシャ/ペルシア variant"),
    # 425 q00624 基礎者 きそもの vs きそしゃ: 者=しゃ in compound -> A
    ("q00624", 0): ("A", "者=しゃ in compound 基礎者"),
    # 426 q00624 日本 -> B
    ("q00624", 2): ("B", "日本 reading nuance"),
    # 427 q00624 ペルシャ/ペルシア -> B
    ("q00624", 3): ("B", "ペルシャ/ペルシア variant"),
    # 428 q00627 一八六七年 -> 1867年: pykakasi reads as kanji digits 'ichihatsurokushichi' wrong;
    # actual '1867nen' standard. -> A
    ("q00627", 2): ("A", "Japanese kanji digits read as せんはっぴゃくろくじゅうしち; actual uses digits"),
    # 429 q00629 大戦 たいせん standard; actual drops 'sekai' segment -> A
    ("q00629", 1): ("A", "Actual drops 世界 segment"),
    # 430 q00630 改革者 かいかくしゃ standard; pykakasi 'mono' wrong -> A (actual correct)
    ("q00630", 2): ("A", "者=しゃ standard; actual correct"),
    # 431 q00633 ノルマンディー上陸 vs actual rewrites as 'dorunodansen' nonsensical -> A
    ("q00633", 0): ("A", "Actual renders ノルマンディー as nonsensical 'dorunodansen'"),
    # 432 q00633 日本のパールハーバー: 日本 -> B
    ("q00633", 1): ("B", "日本 reading nuance"),
    # 433 q00633 大戦 vs actual drops 'sekai' -> A
    ("q00633", 3): ("A", "Actual drops 世界 segment"),
    # 434 q00635 創始者 そうししゃ vs actual 'sosha' (drops 'shi'): -> A
    ("q00635", 0): ("A", "創始者=そうししゃ; actual drops 'shi'"),
    # 435 q00636 ギリシャ vs ギリシア -> B
    ("q00636", 0): ("B", "ギリシャ/ギリシア variant"),
    # 436 q00636 ペルシャ/ペルシア -> B
    ("q00636", 2): ("B", "ペルシャ/ペルシア variant"),
    # 437 q00639 ヨーロッパ vs 欧州(おうしゅう): actual rewrites ヨーロッパ as 欧州 -> A
    ("q00639", 0): ("A", "Actual rewrites ヨーロッパ as 欧州 - text mismatched"),
    # 438 q00639 ギリシャ/ギリシア -> B
    ("q00639", 1): ("B", "ギリシャ/ギリシア variant"),
    # 439 q00639 ヨーロッパへのヴァイキング vs 英国へのバイキング: actual rewrites text -> A
    ("q00639", 3): ("A", "Actual rewrites ヨーロッパ as 英国 - text mismatched"),
    # 440 q00641 指導者 hira しどうしゃ vs actual 'doshatodairyo' (drops 'shi'): -> A
    ("q00641", 0): ("A", "Actual drops 'shi' from 指導者"),
    # 441 q00641 革命家 hira かくめいいえ wrong (家=いえ); actual かくめいか (家=か in 〜家 standard) -> A (actual correct)
    ("q00641", 3): ("A", "家 in 革命家=か standard; actual correct"),
    # 442 q00643 同盟形成 hira けいせい vs actual けっせい: 形成=けいせい standard; actual wrong -> A (actual is wrong)
    # Wait: pykakasi expected 'keisei' (correct), actual 'kessei' (結成?). ja says 形成 so けいせい is right.
    # actual uses 結成 reading -> mismatched text -> A (existing wrong)
    ("q00643", 3): ("A", "形成=けいせい; actual reads as 結成"),
    # 443 q00644 基礎者 -> A
    ("q00644", 0): ("A", "基礎者=きそしゃ standard"),
    # 444 q00644 日本 -> B
    ("q00644", 1): ("B", "日本 reading nuance"),
    # 445 q00646 ギリシャ/ギリシア -> B
    ("q00646", 2): ("B", "ギリシャ/ギリシア variant"),
    # 446 q00646 ペルシャ/ペルシア -> B
    ("q00646", 3): ("B", "ペルシャ/ペルシア variant"),
    # 447 q00647 中国日戦争 -> 中日戦争(ちゅうにち): actual drops 'goku' -> intentional standard term?
    # ja text has 国 (中国日戦争 is unusual; standard is 中日戦争). actual matches standard term -> C
    ("q00647", 2): ("C", "Actual reads as standard 中日戦争 - text contains stray 国"),
    # 448 q00647 イギリス領インド vs 英領インド: actual rewrites as 英領 -> A
    ("q00647", 3): ("A", "Actual rewrites イギリス領 as 英領 - text mismatched"),
    # 449 q00648 第16代 hira 'だい16だい' twice vs actual omits leading 'dai': both grammatical -> B
    ("q00648", 0): ("B", "第16代 reading with optional leading 第; both okay"),
    # 450 q00648 世界線 vs 世界大戦: actual rewrites 線→大戦 -> A
    ("q00648", 2): ("A", "Actual rewrites 世界線 as 世界大戦/前 - text mismatched"),
    # 451 q00648 same -> A
    ("q00648", 3): ("A", "Actual rewrites 世界線 - text mismatched"),
    # 452 q00650 原爆 vs 原子爆弾: actual expands -> A
    ("q00650", 0): ("A", "Actual rewrites 原爆 as 原子爆弾 - text mismatched"),
    # 453 q00650 マンハッタン島 vs マンハッタン都(と): actual reads 'manhattanto' -> A
    ("q00650", 1): ("A", "島=しま; actual reads as 都(と) - wrong"),
    # 454 q00650 探査 vs 探索: actual adds 'ku' -> A
    ("q00650", 3): ("A", "宇宙探査; actual reads as 探索 with extra 'ku'"),
    # 455 q00652 武家政権 hira ぶけせいけん vs actual 'bugatekuseiken': actual mis-segments -> A
    ("q00652", 0): ("A", "武家=ぶけ; actual mis-reads as 'bugateku'"),
}
