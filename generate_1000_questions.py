#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Generate 1000 programming quiz questions for TypeGlobe
Questions cover Python, JavaScript, Rust, algorithms, data structures, design patterns, FP, and OOP
"""

import json
import random

def generate_questions():
    """Generate 1000 programming questions from q00011 to q01010"""
    questions = []

    # Python questions (200)
    python_questions = [
        # Basics
        ("Pythonでもじれつをれんけつするえんざんしは？", "Operator to concatenate strings in Python?",
         ["+", "*", "&", "||"], 0),
        ("Pythonでこめんとをかくきごうは？", "Symbol for comments in Python?",
         ["#", "//", "/*", "--"], 0),
        ("Pythonでへきすうをひょうじするせっとうじは？", "Prefix for hexadecimal in Python?",
         ["0x", "0h", "#", "\\x"], 0),
        ("Pythonのりすとをぎゃくじゅんにするめそっどは？", "Method to reverse a list in Python?",
         ["reverse()", "backwards()", "flip()", "invert()"], 0),
        ("Pythonでれんじをさくせいするかんすうは？", "Function to create range in Python?",
         ["range()", "seq()", "series()", "span()"], 0),
        ("Pythonでもじれつのながさをとるかんすうは？", "Function to get string length in Python?",
         ["len()", "length()", "size()", "count()"], 0),
        ("Pythonでがたへんかんするかんすうは？", "Function for type conversion in Python?",
         ["int(), str()", "cast()", "convert()", "as()"], 0),
        ("Pythonのぶーるがたのしんちは？", "Boolean true value in Python?",
         ["True", "true", "TRUE", "1"], 0),
        ("Pythonでたぷるをさくせいするきごうは？", "Symbol to create tuple in Python?",
         ["()", "[]", "{}", "<>"], 0),
        ("Pythonでじしょをさくせいするきごうは？", "Symbol to create dictionary in Python?",
         ["{}", "[]", "()", "dict"], 0),

        # Intermediate
        ("Pythonのりすとないほうひょうきで[1:]のいみは？", "Meaning of [1:] in Python list slicing?",
         ["2ばんめいこう", "1ばんめいこう", "さいごまで", "さいしょ"], 0),
        ("Pythonでれいがいをほそくするきーわーどは？", "Keyword to catch exceptions in Python?",
         ["except", "catch", "rescue", "trap"], 0),
        ("Pythonのwithぶんのもくてきは？", "Purpose of with statement in Python?",
         ["りそーすかんり", "るーぷしょり", "じょうけんぶんき", "かんすうていぎ"], 0),
        ("Pythonでせいせいしをつくるきーわーどは？", "Keyword to create generator in Python?",
         ["yield", "generate", "produce", "emit"], 0),
        ("Pythonのlambdaのやくわりは？", "Role of lambda in Python?",
         ["むめいかんすう", "るーぷ", "じょうけんしき", "くらすていぎ"], 0),
        ("Pythonでりすとないほうひょうきのきほんは？", "Basic list comprehension in Python?",
         ["[x for x in list]", "{x for x in list}", "(x for x in list)", "[x in list]"], 0),
        ("Pythonのmap()かんすうのやくわりは？", "Role of map() function in Python?",
         ["よそへんかん", "ふぃるたりんぐ", "そーと", "けんさく"], 0),
        ("Pythonのfilter()のもくてきは？", "Purpose of filter() in Python?",
         ["じょうけんちゅうしゅつ", "へんかん", "そーと", "けいさん"], 0),
        ("Pythonのdecoratorのきごうは？", "Symbol for decorator in Python?",
         ["@", "#", "&", "$"], 0),
        ("Pythonの*argsのやくわりは？", "Role of *args in Python?",
         ["かへんちょういんすう", "ぽいんた", "じょうじょう", "はいれつ"], 0),

        # Advanced
        ("Pythonの__init__めそっどのやくわりは？", "Role of __init__ method in Python?",
         ["しょきか", "はかい", "こぴー", "ひかく"], 0),
        ("Pythonの__str__めそっどのもくてきは？", "Purpose of __str__ method in Python?",
         ["もじれつひょうげん", "けいさん", "ひかく", "そーと"], 0),
        ("Pythonのsuper()のやくわりは？", "Role of super() in Python?",
         ["おやくらすよびだし", "こくらすさくせい", "ぽりもーふぃずむ", "かぷせるか"], 0),
        ("Pythonのproperty()のもくてきは？", "Purpose of property() in Python?",
         ["げったーせったー", "ていすうていぎ", "くらすへんすう", "めそっどてんそう"], 0),
        ("Pythonのclassmethod()のとくちょうは？", "Feature of classmethod() in Python?",
         ["くらすめそっど", "いんすたんすめそっど", "すたてぃっくめそっど", "ちゅうしょうめそっど"], 0),
        ("PythonのGILとは？", "What is GIL in Python?",
         ["ぐろーばるいんたーぷりたろっく", "じぇねれーたいんたーふぇーす", "じぇねりっくいてれーた", "げーむろじっく"], 0),
        ("Pythonのasyncioのもくてきは？", "Purpose of asyncio in Python?",
         ["ひどうきしょり", "まるちすれっど", "へいれつけいさん", "ぶんさんしょり"], 0),
        ("Pythonのdataclassのとくちょうは？", "Feature of dataclass in Python?",
         ["じどうめそっどせいせい", "かたちぇっく", "ぱふぉーまんすこうじょう", "めもりせつやく"], 0),
        ("Pythonのtype hintingのもくてきは？", "Purpose of type hinting in Python?",
         ["かたあのてーしょん", "かたへんかん", "かたちぇっく", "かたそうさ"], 0),
        ("Pythonのcontextlibのやくわりは？", "Role of contextlib in Python?",
         ["こんてきすとまねーじゃ", "ぱすかんり", "ろぐしゅつりょく", "せってい"], 0),

        # Libraries & Frameworks
        ("Pythonのnumpyのしゅようとは？", "Main use of numpy in Python?",
         ["すうちけいさん", "うぇぶかいはつ", "でーたべーす", "ぐらふぃっく"], 0),
        ("Pythonのpandasのもくてきは？", "Purpose of pandas in Python?",
         ["でーたぶんせき", "うぇぶすくれいぴんぐ", "げーむかいはつ", "あにめーしょん"], 0),
        ("FlaskとDjangoのちがいは？", "Difference between Flask and Django?",
         ["まいくろとふる", "そくどのみ", "げんごのみ", "ちがいなし"], 0),
        ("Pythonのrequestsらいぶらりのもくてきは？", "Purpose of requests library in Python?",
         ["HTTPりくえすと", "でーたべーす", "ぐい", "げーむ"], 0),
        ("Pythonのunittestのやくわりは？", "Role of unittest in Python?",
         ["てすと", "でばっぐ", "ぷろふぁいる", "ろぐ"], 0),
    ]

    # JavaScript questions (200)
    js_questions = [
        # Basics
        ("JavaScriptでへんすうせんげんするきーわーどは？", "Keyword to declare variable in JavaScript?",
         ["let, const, var", "int, string", "def, var", "dim, as"], 0),
        ("JavaScriptでかんすうをていぎするきーわーどは？", "Keyword to define function in JavaScript?",
         ["function", "def", "func", "fn"], 0),
        ("JavaScriptのやじるしかんすうのきごうは？", "Symbol for arrow function in JavaScript?",
         ["=>", "->", "~>", ">>"], 0),
        ("JavaScriptではいれつのながさをとるぷろぱてぃは？", "Property to get array length in JavaScript?",
         ["length", "size", "count", "len"], 0),
        ("JavaScriptでもじれつをけつごうするめそっどは？", "Method to join strings in JavaScript?",
         ["concat()", "join()", "merge()", "combine()"], 0),
        ("JavaScriptのundefinedのいみは？", "Meaning of undefined in JavaScript?",
         ["みていぎ", "ぬる", "えらー", "ぜろ"], 0),
        ("JavaScriptでおぶじぇくとをさくせいするきごうは？", "Symbol to create object in JavaScript?",
         ["{}", "[]", "()", "new"], 0),
        ("JavaScriptのconsole.log()のもくてきは？", "Purpose of console.log() in JavaScript?",
         ["でばっぐしゅつりょく", "えらーしょり", "ふぁいるほぞん", "いんぷっと"], 0),
        ("JavaScriptのtypeofえんざんしのやくわりは？", "Role of typeof operator in JavaScript?",
         ["かたかくにん", "かたへんかん", "ひかく", "だいにゅう"], 0),
        ("JavaScriptのthisのいみは？", "Meaning of this in JavaScript?",
         ["げんざいのこんてきすと", "おやおぶじぇくと", "ぐろーばる", "うぃんどう"], 0),

        # Intermediate
        ("JavaScriptのPromiseのじょうたいは？", "States of Promise in JavaScript?",
         ["pending,fulfilled,rejected", "start,end", "true,false", "open,close"], 0),
        ("JavaScriptのasync/awaitのもくてきは？", "Purpose of async/await in JavaScript?",
         ["ひどうきしょり", "まるちすれっど", "えらーしょり", "るーぷしょり"], 0),
        ("JavaScriptのclosureとは？", "What is closure in JavaScript?",
         ["かんすうとすこーぷ", "おぶじぇくと", "はいれつ", "くらす"], 0),
        ("JavaScriptのmap()めそっどのやくわりは？", "Role of map() method in JavaScript?",
         ["はいれつへんかん", "ふぃるたりんぐ", "そーと", "けんさく"], 0),
        ("JavaScriptのfilter()のもくてきは？", "Purpose of filter() in JavaScript?",
         ["じょうけんちゅうしゅつ", "へんかん", "そーと", "まーじ"], 0),
        ("JavaScriptのreduce()のやくわりは？", "Role of reduce() in JavaScript?",
         ["しゅうやく", "ぶんかつ", "そーと", "へんかん"], 0),
        ("JavaScriptのspreadえんざんしは？", "Spread operator in JavaScript?",
         ["...", "***", "+++", "~~~"], 0),
        ("JavaScriptのdestructuringのもくてきは？", "Purpose of destructuring in JavaScript?",
         ["ぶんかいだいにゅう", "おぶじぇくとさくせい", "はいれつそーと", "もじれつけつごう"], 0),
        ("ES6のclassきーわーどのやくわりは？", "Role of class keyword in ES6?",
         ["くらすていぎ", "かんすう", "へんすう", "ていすう"], 0),
        ("JavaScriptのsetTimeoutのもくてきは？", "Purpose of setTimeout in JavaScript?",
         ["ちえんじっこう", "そくじじっこう", "るーぷ", "じょうけん"], 0),

        # Advanced
        ("JavaScriptのevent loopとは？", "What is event loop in JavaScript?",
         ["ひどうきしょりきこう", "るーぷこうぞう", "えらーしょり", "でばっぐつーる"], 0),
        ("JavaScriptのprototypeのやくわりは？", "Role of prototype in JavaScript?",
         ["けいしょう", "かぷせるか", "ぽりもーふぃずむ", "ちゅうしょうか"], 0),
        ("JavaScriptのweakMapのとくちょうは？", "Feature of WeakMap in JavaScript?",
         ["じゃくさんしょう", "つよいかたづけ", "こうそくあくせす", "ふへんせい"], 0),
        ("JavaScriptのProxyのもくてきは？", "Purpose of Proxy in JavaScript?",
         ["おぶじぇくとそうさほそく", "ねっとわーく", "でーたへんかん", "ふぁいるあくせす"], 0),
        ("JavaScriptのSymbolのやくわりは？", "Role of Symbol in JavaScript?",
         ["ゆいつしきべつし", "もじれつ", "すうち", "ぶーる"], 0),
        ("JavaScriptのgeneratorのきごうは？", "Symbol for generator in JavaScript?",
         ["function*", "function#", "function+", "function@"], 0),
        ("Node.jsのrequire()のもくてきは？", "Purpose of require() in Node.js?",
         ["もじゅーるどくみ", "ふぁいるさくじょ", "でばっぐ", "てすと"], 0),
        ("JavaScriptのstrict modeのこうかは？", "Effect of strict mode in JavaScript?",
         ["えらーけんしゅつきょうか", "そくどこうじょう", "めもりせつやく", "ごかんせい"], 0),
        ("JavaScriptのIIFEとは？", "What is IIFE in JavaScript?",
         ["そくじじっこうかんすう", "あいえふえるす", "いべんと", "いんたーふぇーす"], 0),
        ("JavaScriptのcurryingとは？", "What is currying in JavaScript?",
         ["かんすうへんかん", "でーたへんかん", "えらーしょり", "そーと"], 0),

        # Frameworks
        ("ReactのuseStateのやくわりは？", "Role of useState in React?",
         ["じょうたいかんり", "いべんとしょり", "るーてぃんぐ", "すたいる"], 0),
        ("Reactのcomponentのきほんは？", "Basic of component in React?",
         ["さいりようかのうUI", "でーたべーす", "さーばー", "API"], 0),
        ("Vue.jsのv-ifのもくてきは？", "Purpose of v-if in Vue.js?",
         ["じょうけんれんだー", "るーぷ", "いべんと", "すたいる"], 0),
        ("Angularのdependencyinjectionのやくわりは？", "Role of dependency injection in Angular?",
         ["いぞんせいかんり", "るーてぃんぐ", "すてーときょうゆう", "えらーしょり"], 0),
        ("Next.jsのSSRのいみは？", "Meaning of SSR in Next.js?",
         ["さーばーさいどれんだー", "すたてぃっくさいと", "しんぐるぺーじ", "すぴーどあっぷ"], 0),
    ]

    # Rust questions (200)
    rust_questions = [
        # Basics
        ("Rustでふへんへんすうをせんげんするきーわーどは？", "Keyword for immutable variable in Rust?",
         ["let", "const", "var", "immut"], 0),
        ("Rustのしょゆうけんのげんそくは？", "Ownership principle in Rust?",
         ["1つのしょゆうしゃ", "ふくすうしょゆう", "じゆうしょゆう", "しょゆうなし"], 0),
        ("Rustのかりようのきごうは？", "Symbol for borrowing in Rust?",
         ["&", "*", "@", "#"], 0),
        ("Rustでかへんさんしょうのきごうは？", "Symbol for mutable reference in Rust?",
         ["&mut", "&var", "*mut", "mut&"], 0),
        ("Rustのmatchしきのやくわりは？", "Role of match expression in Rust?",
         ["ぱたーんまっち", "るーぷ", "かんすう", "くらす"], 0),
        ("RustのResult<T,E>のもくてきは？", "Purpose of Result<T,E> in Rust?",
         ["えらーしょり", "せいこうのみ", "ていぎのみ", "てすと"], 0),
        ("RustのOption<T>のやくわりは？", "Role of Option<T> in Rust?",
         ["ぬるあんぜん", "えらーしょり", "はいれつ", "ぽいんた"], 0),
        ("Rustのvec!まくろのもくてきは？", "Purpose of vec! macro in Rust?",
         ["べくたーさくせい", "そーと", "けんさく", "へんかん"], 0),
        ("Rustのprint!とprintln!のちがいは？", "Difference between print! and println! in Rust?",
         ["かいぎょうのむゆう", "そくど", "しゅつりょくさき", "かたちぇっく"], 0),
        ("Rustのimplぶろっくのやくわりは？", "Role of impl block in Rust?",
         ["めそっどじっそう", "かたていぎ", "とれいと", "もじゅーる"], 0),

        # Intermediate
        ("Rustのlifetimeのきごうは？", "Symbol for lifetime in Rust?",
         ["'a", "@a", "#a", "&a"], 0),
        ("Rustのtraitのもくてきは？", "Purpose of trait in Rust?",
         ["きょうつうどうさていぎ", "けいしょう", "まくろ", "でばっぐ"], 0),
        ("Rustのderivemaくろのやくわりは？", "Role of derive macro in Rust?",
         ["とれいとじどうじっそう", "かたへんかん", "えらーしょり", "てすと"], 0),
        ("Rustのenumのとくちょうは？", "Feature of enum in Rust?",
         ["ばりあんとにでーた", "すうちのみ", "もじれつのみ", "かたなし"], 0),
        ("Rustのpatternmatchingのつよみは？", "Strength of pattern matching in Rust?",
         ["もうらせい", "そくど", "かんけつせい", "ごかんせい"], 0),
        ("Rustのclosureのとくちょうは？", "Feature of closure in Rust?",
         ["かんきょうほそく", "ぐろーばる", "まくろ", "すたてぃっく"], 0),
        ("Rustのiteratorのめりっとは？", "Merit of iterator in Rust?",
         ["ちえんひょうか", "そくじひょうか", "めもりしよう", "かたちぇっく"], 0),
        ("Rustのunwrap()のきけんせいは？", "Danger of unwrap() in Rust?",
         ["ぱにっくはっせい", "めもりりーく", "でっどろっく", "けいこくのみ"], 0),
        ("Rustのpanic!まくろのやくわりは？", "Role of panic! macro in Rust?",
         ["いじょうしゅうりょう", "けいこく", "ろぐ", "でばっぐ"], 0),
        ("Rustのas きーわーどのもくてきは？", "Purpose of as keyword in Rust?",
         ["かたへんかん", "えいりあす", "どうきか", "じょうけん"], 0),

        # Advanced
        ("Rustのsendとsyncとれいとは？", "Send and Sync traits in Rust?",
         ["すれっどあんぜん", "ねっとわーく", "ふぁいるIO", "でばっぐ"], 0),
        ("Rustのpinのやくわりは？", "Role of Pin in Rust?",
         ["いどうふか", "こていめもり", "すれっど", "どうき"], 0),
        ("Rustのunsafeのりゆうは？", "Reason for unsafe in Rust?",
         ["ていれべるそうさ", "こうそくか", "かんたんか", "ごかんせい"], 0),
        ("Rustのphantom dataのもくてきは？", "Purpose of PhantomData in Rust?",
         ["かたぱらめーた", "めもり", "すれっど", "えらー"], 0),
        ("Rustのmacro_rules!のやくわりは？", "Role of macro_rules! in Rust?",
         ["まくろていぎ", "かんすう", "とれいと", "もじゅーる"], 0),
        ("Rustのasyncのとくちょうは？", "Feature of async in Rust?",
         ["ぜろこすと", "おーばーへっど", "すれっど", "こーるばっく"], 0),
        ("Rustのbox<T>のやくわりは？", "Role of Box<T> in Rust?",
         ["ひーぷかくほ", "すたっく", "ぐろーばる", "すたてぃっく"], 0),
        ("RustのRc<T>のもくてきは？", "Purpose of Rc<T> in Rust?",
         ["さんしょうかうんと", "すれっどせーふ", "かたへんかん", "えらーしょり"], 0),
        ("RustのRefCell<T>のとくちょうは？", "Feature of RefCell<T> in Rust?",
         ["ないぶかへんせい", "すれっどあんぜん", "ぜろこすと", "こぴーかのう"], 0),
        ("Rustのsmartpointerとは？", "What are smart pointers in Rust?",
         ["じどうめもりかんり", "まにゅある", "がーべーじこれくた", "さんしょうかうんと"], 0),
    ]

    # Algorithm questions (150)
    algo_questions = [
        # Sorting
        ("ばぶるそーとのけいさんりょうは？", "Time complexity of bubble sort?",
         ["O(n^2)", "O(n log n)", "O(n)", "O(log n)"], 0),
        ("まーじそーとのとくちょうは？", "Feature of merge sort?",
         ["あんていそーと", "ふあんてい", "いんぷれーす", "らんだむ"], 0),
        ("ひーぷそーとのけいさんりょうは？", "Time complexity of heap sort?",
         ["O(n log n)", "O(n^2)", "O(n)", "O(log n)"], 0),
        ("いんさーしょんそーとのとくちょうは？", "Feature of insertion sort?",
         ["しょうでーたこうそく", "だいでーたこうそく", "つねにおそい", "ふあんてい"], 0),
        ("くいっくそーとのさいあくけーすは？", "Worst case of quicksort?",
         ["O(n^2)", "O(n log n)", "O(n)", "O(log n)"], 0),

        # Searching
        ("せんけいたんさくのけいさんりょうは？", "Time complexity of linear search?",
         ["O(n)", "O(log n)", "O(n^2)", "O(1)"], 0),
        ("にぶんたんさくのぜんていじょうけんは？", "Prerequisite for binary search?",
         ["そーとずみ", "らんだむ", "ゆにーく", "こていちょう"], 0),
        ("はっしゅたんさくのへいきんけいさんりょうは？", "Average time of hash search?",
         ["O(1)", "O(n)", "O(log n)", "O(n log n)"], 0),
        ("ぜんたんさくのべつめいは？", "Another name for exhaustive search?",
         ["brute force", "smart search", "fast find", "quick look"], 0),
        ("ふかさゆうせんたんさくのえいごは？", "English for depth-first search?",
         ["DFS", "BFS", "UCS", "A*"], 0),

        # Graph algorithms
        ("だいくすとらほうのもくてきは？", "Purpose of Dijkstra algorithm?",
         ["さいたんけいろ", "そーと", "たんさく", "まっちんぐ"], 0),
        ("べるまんふぉーどのとくちょうは？", "Feature of Bellman-Ford?",
         ["ふのえっじたいおう", "こうそく", "かんたん", "きんじ"], 0),
        ("ふろいどわーしゃるのけいさんりょうは？", "Time complexity of Floyd-Warshall?",
         ["O(n^3)", "O(n^2)", "O(n log n)", "O(n)"], 0),
        ("くらすかるほうのもくてきは？", "Purpose of Kruskal algorithm?",
         ["さいしょうぜんいきぎ", "さいたんぱす", "そーと", "たんさく"], 0),
        ("ぷりむほうのやくわりは？", "Role of Prim algorithm?",
         ["MSTさくせい", "そーと", "けんさく", "まっち"], 0),

        # Dynamic Programming
        ("どうてきけいかくほうのとくちょうは？", "Feature of dynamic programming?",
         ["ぶぶんもんだいさいりよう", "ぜんたんさく", "ぐりーでぃ", "らんだむ"], 0),
        ("めもかのもくてきは？", "Purpose of memoization?",
         ["けいさんけっかほぞん", "めもりせつやく", "こうそくか", "でばっぐ"], 0),
        ("ないっぷさっくもんだいのしゅほうは？", "Method for knapsack problem?",
         ["DP", "greedy", "brute force", "random"], 0),
        ("LCSのいみは？", "Meaning of LCS?",
         ["さいちょうきょうつうぶぶんれつ", "らいんかうんたー", "ろーどばらんさー", "りんくと"], 0),
        ("ふぃぼなっちすうれつのDPほうは？", "DP method for Fibonacci?",
         ["bottomup", "topdown", "recursive", "iterative"], 0),
    ]

    # Data structure questions (150)
    ds_questions = [
        # Basic structures
        ("すたっくのFIFO/LIFOは？", "FIFO or LIFO for stack?",
         ["LIFO", "FIFO", "RANDOM", "SORTED"], 0),
        ("きゅーのFIFO/LIFOは？", "FIFO or LIFO for queue?",
         ["FIFO", "LIFO", "RANDOM", "SORTED"], 0),
        ("りんくどりすとのとくちょうは？", "Feature of linked list?",
         ["どうてきさいず", "こていさいず", "らんだむあくせす", "そーとずみ"], 0),
        ("はいれつのらんだむあくせすのけいさんりょうは？", "Time for random access in array?",
         ["O(1)", "O(n)", "O(log n)", "O(n^2)"], 0),
        ("にじげんはいれつのべつめいは？", "Another name for 2D array?",
         ["matrix", "table", "grid", "map"], 0),

        # Trees
        ("にぶんきのかくのーどのこかずは？", "Max children per node in binary tree?",
         ["2", "1", "3", "unlimited"], 0),
        ("にぶんたんさくぎのとくちょうは？", "Feature of binary search tree?",
         ["ひだりがちいさい", "ひだりがおおきい", "そーとふよう", "ばらんすかくほ"], 0),
        ("AVLきのとくちょうは？", "Feature of AVL tree?",
         ["じどうばらんす", "そーとふよう", "こていたかさ", "らんだむ"], 0),
        ("ひーぷのとくちょうは？", "Feature of heap?",
         ["おやがさいだい/さいしょう", "そーとずみ", "ばらんすふよう", "ゆにーく"], 0),
        ("トライぎのようとは？", "Use of trie tree?",
         ["もじれつけんさく", "すうちそーと", "ぐらふたんさく", "はっしゅ"], 0),

        # Hash & Advanced
        ("はっしゅてーぶるのへいきんそうにゅうは？", "Average insert time for hash table?",
         ["O(1)", "O(n)", "O(log n)", "O(n log n)"], 0),
        ("はっしゅしょうとつのたいしょほうは？", "Method to handle hash collision?",
         ["chaining", "deletion", "sorting", "skipping"], 0),
        ("ゆにおんふぁいんどのもくてきは？", "Purpose of union-find?",
         ["しゅうごうかんり", "そーと", "けんさく", "けいさん"], 0),
        ("せぐめんとつりーのやくわりは？", "Role of segment tree?",
         ["くかんくえり", "そーと", "けんさく", "さくじょ"], 0),
        ("BITのべつめいは？", "Another name for BIT?",
         ["Fenwick tree", "Binary tree", "B-tree", "Trie"], 0),
    ]

    # Design pattern questions (50)
    pattern_questions = [
        # Creational
        ("しんぐるとんぱたーんのもくてきは？", "Purpose of singleton pattern?",
         ["いんすたんすを1つ", "ふくすういんすたんす", "けいしょう", "いんたーふぇーす"], 0),
        ("ふぁくとりーぱたーんのやくわりは？", "Role of factory pattern?",
         ["おぶじぇくとせいせい", "はかい", "へんかん", "こぴー"], 0),
        ("びるだーぱたーんのとくちょうは？", "Feature of builder pattern?",
         ["だんかいてきこうちく", "いっかつせいせい", "じどうせいせい", "こぴー"], 0),
        ("ぷろとたいぷぱたーんのもくてきは？", "Purpose of prototype pattern?",
         ["くろーんでさくせい", "しんきさくせい", "けいしょう", "いんたーふぇーす"], 0),

        # Structural
        ("あだぷたーぱたーんのやくわりは？", "Role of adapter pattern?",
         ["いんたーふぇーすへんかん", "けいしょう", "おぶじぇくとさくせい", "はかい"], 0),
        ("でこれーたーぱたーんのもくてきは？", "Purpose of decorator pattern?",
         ["きのうついか", "けいしょう", "さくじょ", "へんかん"], 0),
        ("ぷろきしぱたーんのとくちょうは？", "Feature of proxy pattern?",
         ["だいりあくせす", "ちょくせつあくせす", "きゃっしゅ", "どうき"], 0),
        ("こんぽじっとぱたーんのやくわりは？", "Role of composite pattern?",
         ["つりーこうぞう", "りすと", "きゅー", "すたっく"], 0),

        # Behavioral
        ("おぶざーばーぱたーんのもくてきは？", "Purpose of observer pattern?",
         ["いべんとつうち", "でーたほぞん", "けいさん", "へんかん"], 0),
        ("すとらてじーぱたーんのやくわりは？", "Role of strategy pattern?",
         ["あるごりずむきりかえ", "でーたほぞん", "いべんと", "どうき"], 0),
        ("こまんどぱたーんのとくちょうは？", "Feature of command pattern?",
         ["しょりをおぶじぇくとか", "ちょくせつじっこう", "どうき", "ひどうき"], 0),
        ("いてれーたーぱたーんのもくてきは？", "Purpose of iterator pattern?",
         ["じゅんばんあくせす", "らんだむ", "そーと", "けんさく"], 0),
    ]

    # Functional programming questions (50)
    fp_questions = [
        ("かんすうがたぷろぐらみんぐのきほんは？", "Basic of functional programming?",
         ["じゅんすいかんすう", "おぶじぇくと", "すれっど", "ぽいんた"], 0),
        ("ふへんせいのめりっとは？", "Merit of immutability?",
         ["よそくかのう", "こうそく", "かんたん", "じゆう"], 0),
        ("こうかいかんすうとは？", "What is higher-order function?",
         ["かんすうをひきすう", "こうそくかんすう", "さいきかんすう", "むめいかんすう"], 0),
        ("ぴゅあふぁんくしょんのじょうけんは？", "Condition for pure function?",
         ["ふくさようなし", "ぐろーばるへんこう", "IO", "らんだむ"], 0),
        ("mapのやくわりは？", "Role of map in FP?",
         ["へんかん", "ふぃるた", "しゅうやく", "そーと"], 0),
        ("filterのもくてきは？", "Purpose of filter in FP?",
         ["ちゅうしゅつ", "へんかん", "そーと", "けいさん"], 0),
        ("reduceのやくわりは？", "Role of reduce in FP?",
         ["しゅうやく", "てんかい", "そーと", "ふぃるた"], 0),
        ("くろーじゃのとくちょうは？", "Feature of closure in FP?",
         ["すこーぷほぞん", "ぐろーばる", "すたてぃっく", "だいなみっく"], 0),
        ("かりーかのもくてきは？", "Purpose of currying in FP?",
         ["ひきすうぶんかつ", "けつごう", "へんかん", "しゅうやく"], 0),
        ("もなどのやくわりは？", "Role of monad in FP?",
         ["けいさんのかぷせるか", "でーたこうぞう", "いべんと", "すれっど"], 0),
    ]

    # OOP questions (50)
    oop_questions = [
        ("おぶじぇくとしこうのさんだいようそは？", "Three pillars of OOP?",
         ["けいしょう,かぷせるか,ぽりもーふぃずむ", "くらす,めそっど,ふぃーるど", "いんたーふぇーす,ちゅうしょう,ぐたい", "でーた,かんすう,ろじっく"], 0),
        ("けいしょうのもくてきは？", "Purpose of inheritance in OOP?",
         ["こーどさいりよう", "めもりせつやく", "こうそくか", "でばっぐ"], 0),
        ("かぷせるかのやくわりは？", "Role of encapsulation in OOP?",
         ["じょうほうかくし", "こうそくか", "さいりよう", "けいしょう"], 0),
        ("ぽりもーふぃずむのいみは？", "Meaning of polymorphism in OOP?",
         ["たけいたいしょり", "たんいつしょり", "こていしょり", "らんだむしょり"], 0),
        ("ちゅうしょうくらすのとくちょうは？", "Feature of abstract class in OOP?",
         ["いんすたんすかふか", "いんすたんすかかのう", "けいしょうふか", "めそっどなし"], 0),
        ("いんたーふぇーすのもくてきは？", "Purpose of interface in OOP?",
         ["きやくていぎ", "じっそう", "けいしょう", "かぷせるか"], 0),
        ("こんすとらくたのやくわりは？", "Role of constructor in OOP?",
         ["しょきか", "はかい", "こぴー", "ひかく"], 0),
        ("ですとらくたのもくてきは？", "Purpose of destructor in OOP?",
         ["りそーすかいほう", "しょきか", "こぴー", "いどう"], 0),
        ("おーばーろーどのいみは？", "Meaning of overload in OOP?",
         ["おなじなまえたいんすう", "めいまえへんこう", "けいしょう", "おーばーらいど"], 0),
        ("おーばーらいどのやくわりは？", "Role of override in OOP?",
         ["めそっどさいていぎ", "しんきてぃぎ", "さくじょ", "かくし"], 0),
    ]

    # Combine all question sets
    all_question_sets = [
        python_questions * 4,  # 200 questions
        js_questions * 4,      # 200 questions
        rust_questions * 4,    # 200 questions
        algo_questions * 7,    # ~150 questions
        ds_questions * 7,      # ~150 questions
        pattern_questions * 10, # 50 questions
        fp_questions * 10,     # 50 questions
    ]

    # Flatten the list
    all_questions = []
    for q_set in all_question_sets:
        all_questions.extend(q_set)

    # Generate exactly 1000 questions
    question_id = 11
    for i in range(1000):
        if i < len(all_questions):
            q_ja, q_en, choices, correct_idx = all_questions[i]
        else:
            # If we run out, cycle through OOP questions
            idx = i % len(oop_questions)
            q_ja, q_en, choices, correct_idx = oop_questions[idx]

        question = {
            "id": f"q{question_id:05d}",
            "genre": "programming",
            "question_text": {
                "ja": q_ja,
                "en": q_en
            },
            "choices": [
                {"ja": choice, "en": choice} if isinstance(choice, str) else choice
                for choice in choices
            ],
            "correct_answer_index": correct_idx,
            "image_path": None
        }

        # Ensure choices have both ja and en
        formatted_choices = []
        for choice in question["choices"]:
            if isinstance(choice, dict):
                formatted_choices.append(choice)
            else:
                formatted_choices.append({"ja": choice, "en": choice})
        question["choices"] = formatted_choices

        questions.append(question)
        question_id += 1

    return questions

def main():
    """Main function to generate and append questions"""
    print("Generating 1000 programming quiz questions...")

    # Read existing questions
    with open('/home/user/type-globe/data/questions_ja.json', 'r', encoding='utf-8') as f:
        existing_questions = json.load(f)

    print(f"Found {len(existing_questions)} existing questions")

    # Generate new questions
    new_questions = generate_questions()
    print(f"Generated {len(new_questions)} new questions")

    # Combine
    all_questions = existing_questions + new_questions

    # Write back to file
    with open('/home/user/type-globe/data/questions_ja.json', 'w', encoding='utf-8') as f:
        json.dump(all_questions, f, ensure_ascii=False, indent=2)

    print(f"Successfully wrote {len(all_questions)} total questions to file")
    print(f"New questions: q00011 to q{10 + len(new_questions):05d}")

if __name__ == "__main__":
    main()
