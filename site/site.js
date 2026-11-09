// The theme switch: shows the saved choice, applies a new one and remembers it, like the web app.
(() => {
    const key = 'uma-stable-match:theme';
    const root = document.documentElement;
    let saved = 'system';
    try {
        saved = localStorage.getItem(key) || 'system';
    } catch {
        saved = 'system';
    }
    for (const input of document.querySelectorAll('.theme-switch input')) {
        input.checked = input.value === saved;
        input.addEventListener('change', () => {
            if (input.value === 'system') {
                delete root.dataset.theme;
            } else {
                root.dataset.theme = input.value;
            }
            try {
                if (input.value === 'system') {
                    localStorage.removeItem(key);
                } else {
                    localStorage.setItem(key, input.value);
                }
            } catch {
                return;
            }
        });
    }
})();

// Run locally, the website is on site.localhost and the app on localhost of the same server.
if (location.hostname === 'site.localhost') {
    const app = `${location.protocol}//localhost${location.port ? `:${location.port}` : ''}`;
    for (const link of document.querySelectorAll('a[href^="https://app.umastable.com"]')) {
        link.href = app + link.getAttribute('href').slice('https://app.umastable.com'.length);
    }
}

// The language button. English is the page as written; Japanese swaps in the text below. The
// button only shows when the server's ENABLE_JAPANESE is on.
fetch('/api/v1/features')
    .then((response) => (response.ok ? response.json() : {}))
    .catch(() => ({}))
    .then((features) => {
    const JAPANESE_ENABLED = features.japanese === true;

    const key = 'uma-stable-match:language';
    // A cookie, so app.umastable.com shares the choice.
    const cookie = 'uma-stable-match-language';
    const ja = {
        'meta.title': 'Uma Stable Match: ウマ娘 サークル検索・サークルマッチ',
        'meta.description': 'ウマ娘のサークル検索・マッチングツール(無料)。あなたの1日のファン獲得数と、空きのある全サークルのメンバーを比べ、ぴったり合うサークルを順位付けします。',
        'nav.label': 'メインメニュー',
        'nav.how': 'しくみ',
        'nav.faq': 'よくある質問',
        'nav.pages': 'ページ',
        'nav.home': 'ホーム',
        'nav.finder': 'サークル検索',
        'nav.wishlist': 'ウィッシュリスト',
        'nav.applications': 'サークルマスターの方へ',
        'nav.board': '募集掲示板',
        'nav.account': 'アカウント',
        'lang.switch': '英語に切り替え (English)',
        'theme.label': 'テーマ',
        'theme.system': 'システム',
        'theme.light': 'ライト',
        'theme.dark': 'ダーク',
        'hero.title': 'もう迷わない。ウマ娘のぴったりのサークルを見つけよう。',
        'hero.body': 'Uma Stable Matchは、無料のウマ娘サークル検索・マッチングツールです。あなたの1日のファン獲得数と、空きのある全サークルを比べ、ペースが合い、メンバーが偏りなく貢献しているサークルを順位付けします。',
        'hero.cta': 'サークルを探す',
        'hero.how': 'しくみ',
        'see.title': 'あなたにぴったりのサークルを順位付け',
        'see.body': 'トレーナーIDか1日のファン獲得数を入力すると、おすすめのサークルが表示されます。各サークルの順位、空き枠、加入方法、メンバーの中でのあなたの順位も確認できます。',
        'see.alt': 'Uma Stable Matchの結果画面。1日のペースは600万ファン。調べたサークルの概要と、おすすめのサークルが表示されています。各サークルのティア、順位と変動、空き枠、加入方法、メンバーの中でのあなたの順位、1日の中央値、バランス、おんぶ組が確認できます。',
        'see.caption': '結果の例(サークルは架空のものです)。',
        'how.title': 'サークルマッチのしくみ',
        'how.body': 'サークルのランキングでわかるのはサークルの大きさで、あなたに合うかどうかではありません。Uma Stable Matchはメンバーに注目します。',
        'how.pace.title': 'あなたのファン獲得ペースに合わせる',
        'how.pace.body': 'あなたの1日のファン獲得数を、各サークルの現在のメンバーと比べます。いちばん合うのは、あなたが真ん中あたりに入れるサークルです。切られる心配がなく、かといって他のメンバーを支え続ける必要もありません。',
        'how.weight.title': 'みんながしっかり貢献しているメンバー',
        'how.weight.body': 'バランス評価では、ファン数がメンバーにどれだけ均等に分かれているか、おんぶ組が何人いるかを見ます。一部のメンバーが全員を支えているサークルは、順位が下がることがあります。',
        'how.open.title': '空きがあり、活発なサークル',
        'how.open.body': '空きのあるサークルだけを比べます(条件を変えない限り)。現在のメンバーだけを数えるので、抜けたプレイヤーがサークルの数字を狂わせることはありません。',
        'results.title': '選ぶのに必要な情報がすべて',
        'results.body': '上位100件を検索・並べ替えして、サークルのuma.moeページを開き、サークルマスターに連絡できます。',
        'results.alt': '上位の結果の表。各サークルのティア、順位、先月の順位、空き枠、加入方法、1日の中央値、あなたの順位が並び、それぞれに「メンバーを見る」ボタンがあります。',
        'results.caption': '上位の結果の表(サークルは架空のものです)。',
        'field.0.name': 'あなたの順位',
        'field.0.value': '30人中8位',
        'field.0.note': '現在のメンバーの中でのあなたの順位。',
        'field.1.name': '1日あたりの中央値',
        'field.1.value': '5.3M/日',
        'field.1.note': '今月の典型的なメンバーの1日のファン獲得数。',
        'field.2.name': '順位とティア',
        'field.2.value': '163位 ▲30',
        'field.2.note': 'サークルの月間順位、先月からの変動、そしてDからSSまでのティア。',
        'field.3.name': '加入方法',
        'field.3.value': '自由加入 · 承認制',
        'field.3.note': 'すぐに加入できるか、サークルマスターの承認を待つか。募集停止のサークルは、「募集停止」を選んだときだけ表示されます。',
        'field.4.name': 'バランス',
        'field.4.value': '86/100',
        'field.4.note': 'メンバーの貢献がどれだけ均等か。高いほど良い値です。',
        'field.5.name': 'おんぶ組',
        'field.5.value': '27人中0人',
        'field.5.note': '1日のファン獲得数が100万未満、またはサークルの中央値の4分の1未満のメンバー。',
        'apply.title': 'ワンクリックでサークルに応募',
        'apply.body': 'Discordでサインインして、結果からそのまま応募できます。Uma Stable Matchのボットがあるサークルには、あなたのファン獲得ペースと履歴が非公開チケットで届き、応募状況はまとめて確認できます。',
        'apply.alt': '「自分の申請」ページ。4件の応募について、トレーナーID、送信日、有効期限、保留中・承認・辞退の状態が一覧になっています。',
        'apply.caption': '自分の申請(サークルは架空のものです)。',
        'apply.quick.title': 'かんたん応募',
        'apply.quick.body': 'ボットのあるサークルで「かんたん応募」を選ぶだけで応募が送られます。入力フォームもスクリーンショットも不要です。',
        'apply.ticket.title': 'あなたの数字つきチケット',
        'apply.ticket.body': 'サークルマスターは、1日のファン獲得数、今月のファン数、現在のサークルを非公開のDiscordチケットで確認し、承認または辞退します。',
        'apply.track.title': 'すべての応募を確認',
        'apply.track.body': '保留中・承認・辞退の状態と、それぞれの有効期限を確認できます。',
        'apply.cta': 'サークル申請を開く',
        'apply.leaders': 'サークルマスターの方はボットをチェック',
        'wish.title': '次に作るものを一緒に決めよう',
        'wish.body': 'サークルマスター向けのダッシュボードから日本版のトレーナーとサークルまで、使いたいアイデアに投票してください。',
        'wish.alt': 'ウィッシュリストのページ。次に作るものを決めるための見出しと、サークルマスター向け応募ダッシュボードやDiscordボット連携の投票が並んでいます。',
        'wish.caption': 'ウィッシュリスト(投票数は架空のものです)。',
        'wish.cta': 'ウィッシュリストを開く',
        'faq.title': 'よくある質問',
        'faq.0.q': 'ウマ娘で良いサークルはどう探せばいいですか?',
        'faq.0.a': '空きがあり、メンバーのファン獲得ペースがあなたと近く、あなたがメンバーの真ん中あたりに入れて、ほとんどのメンバーがしっかり貢献しているサークルを探しましょう。Uma Stable Matchは、対象の順位帯にあるすべてのサークルでこの比較を行い、いちばん合うサークルを順位付けします。',
        'faq.1.q': '自分のサークルが結果に出ないのはなぜですか?',
        'faq.1.a': '満員(30人中30人)のサークルは、「満員のサークルも含める」にチェックしない限り表示されず、加入できる枠がないため、おすすめにも出ません。現在のサークルも結果には含まれませんが、トレーナーIDで検索すると比較できます。',
        'faq.2.q': '自分のサークルより低いティアのサークルをすすめられるのはなぜですか?',
        'faq.2.a': 'サークルのティアは合計ファン数で決まり、上位メンバーの獲得数に大きく左右されます。Uma Stable Matchはあなたのペースをサークルの典型的なメンバーと比べるので、あなたが真ん中あたりに入れるサークルが選ばれます。上位ティアのサークルの典型的なメンバーよりあなたのペースがかなり低い場合は、1〜2ティア下のサークルが最適になることがあります。',
        'faq.3.q': 'Uma Stable Matchは公式のものですか?',
        'faq.3.a': 'いいえ。非公式の非営利ファンツールで、uma.moeおよび株式会社Cygamesとは提携しておらず、公認・後援も受けていません。',
        'faq.4.q': 'サークルのデータはどこから来ていますか?',
        'faq.4.a': '<a href="https://uma.moe" target="_blank" rel="noopener">uma.moe</a>のサークルとトレーナーのデータです。各サークルは、uma.moeの毎日の更新の少しあとに、1日1回更新されます。',
        'faq.5.q': '無料ですか?',
        'faq.5.a': 'はい。Uma Stable Matchは無料で、MITライセンスのオープンソースです。',
        'faq.6.q': '使うのに何が必要ですか?',
        'faq.6.a': 'トレーナーID(ゲーム内の「メニュー」→「プロフィール」にある9〜12桁の数字)か、1日のファン獲得数です。インストールもアカウント作成も必要ありません。',
        'faq.7.q': 'インストールせずにブラウザで使えますか?',
        'faq.7.a': 'はい。スマートフォンやパソコンで<a href="https://app.umastable.com">app.umastable.com</a>を開き、トレーナーIDか1日のファン獲得数を入力してください。',
        'footer.links': 'プロジェクトのリンク',
        'footer.report': '問題を報告',
        'footer.privacy': 'プライバシーポリシー',
        'footer.terms': 'サービス利用規約',
        'footer.kofi': 'にんじんをおごる',
        'footer.support': '任意の支援はホスティング費用に充てられます。すべての機能は無料のままです。',
        'footer.unofficial': 'Uma Stable Matchは<strong>非公式の非営利ファンツール</strong>です。uma.moeおよび株式会社Cygamesとは提携しておらず、公認・後援も受けていません。',
        'footer.data': 'サークルとトレーナーのデータは<a href="https://uma.moe" target="_blank" rel="noopener noreferrer">uma.moe</a>から取得しており、遅れや欠けがある場合があります。おすすめは判断の目安となる推定であり、特定のプレイヤーやサークルを評価するものではありません。',
        'footer.trademark': '<em>ウマ娘 プリティーダービー</em>の名称、キャラクター、アートワーク、商標は株式会社Cygamesに帰属します。サークルのティアアイコンは© Cygames, Inc.のもので、Cygamesの<a href="https://umamusume.com/fan-createdguide" target="_blank" rel="noopener noreferrer">二次創作ガイドライン</a>に沿って非営利で使用しています。',
    };

    // The page's own English, kept so a switch back restores it.
    const texts = new Map();
    const attributes = new Map();

    function apply(lang) {
        document.documentElement.lang = lang;
        for (const el of document.querySelectorAll('[data-i18n]')) {
            const prop = el.tagName === 'TITLE' ? 'textContent' : 'innerHTML';
            if (!texts.has(el)) {
                texts.set(el, el[prop]);
            }
            el[prop] = (lang === 'ja' && ja[el.dataset.i18n]) || texts.get(el);
        }
        for (const el of document.querySelectorAll('[data-i18n-attr]')) {
            for (const pair of el.dataset.i18nAttr.split(';')) {
                const [name, id] = pair.split(':');
                if (!attributes.has(el)) {
                    attributes.set(el, {});
                }
                const original = attributes.get(el);
                original[name] ??= el.getAttribute(name);
                el.setAttribute(name, (lang === 'ja' && ja[id]) || original[name]);
            }
        }
    }

    if (!JAPANESE_ENABLED) {
        return;
    }
    function load() {
        const shared = document.cookie.split('; ').find((pair) => pair.startsWith(cookie + '='));
        if (shared === cookie + '=en' || shared === cookie + '=ja') {
            return shared.slice(-2);
        }
        try {
            return localStorage.getItem(key) === 'ja' ? 'ja' : 'en';
        } catch {
            return 'en';
        }
    }

    function save(lang) {
        const host = location.hostname;
        const shared = host === 'umastable.com' || host.endsWith('.umastable.com');
        document.cookie = cookie + '=' + lang + '; Path=/; Max-Age=31536000; SameSite=Lax' +
            (shared ? '; Domain=umastable.com' : '') +
            (location.protocol === 'https:' ? '; Secure' : '');
        try {
            localStorage.setItem(key, lang);
        } catch {
            return;
        }
    }

    const saved = load();
    const button = document.getElementById('lang-switch');
    button.hidden = false;
    let lang = saved;
    if (lang === 'ja') {
        apply('ja');
    }
    button.addEventListener('click', () => {
        lang = lang === 'en' ? 'ja' : 'en';
        apply(lang);
        save(lang);
    });
    });
