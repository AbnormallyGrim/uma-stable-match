/**
 * Japanese messages, using the game's own terms: a club is a サークル (circle), a trainer is a
 * トレーナー, and so on. Every message in the English catalog needs one here.
 */

import type { Catalog } from './en';

export const ja: Catalog = {
  'app.name': 'Uma Stable Match',
  'app.heading': 'Uma Stable Match サークル検索',
  'app.found': ({ n }) => `${n}件のサークルが見つかりました。`,

  'nav.label': 'メインメニュー',
  'nav.tour': 'ツアーを見る',

  'language.switch': '英語に切り替え (English)',

  'theme.label': 'テーマ',
  'theme.system': 'システム',
  'theme.light': 'ライト',
  'theme.dark': 'ダーク',

  'time.justNow': 'たった今',
  'time.unknown': '不明',
  'time.seconds': ({ n }) => `${n}秒`,

  'format.perDay': ({ value }) => `${value}/日`,
  'format.place': ({ place, size }) => `${size}人中${place}位`,
  'format.topShare': ({ percent }) => `上位${percent}%`,
  'month.label': ({ year, month }) => `${year}年${month}月`,

  'trend.unknown': '順位不明',
  'trend.new': 'NEW',
  'trend.notRanked': '先月は圏外',
  'trend.up': ({ n }) => `先月より${n}位上昇`,
  'trend.down': ({ n }) => `先月より${n}位下降`,
  'trend.same': '先月と同じ順位',

  'club.full': '満員',
  'club.openSpots': ({ n }) => `空き${n}枠`,
  'club.rank': ({ rank }) => `${rank}位`,
  'club.pick': ({ n }) => `おすすめ${n}番目`,
  'club.discord': 'Discord',
  'club.discordTitle': 'サークル紹介文にDiscordの記載があります',
  'club.youdBe': 'あなたの順位',
  'club.median': '中央値',
  'club.balance': 'バランス',
  'club.passengers': 'おんぶ組',
  'club.top3Share': '上位3人の割合',
  'club.score': 'スコア',
  'club.fit': '適合度',
  'tier.label': ({ tier }) => `サークルティア${tier}`,
  'tier.unknown': 'サークルティア不明',

  'join.open': '自由加入',
  'join.approval': '承認制',
  'join.closed': '募集停止',

  'playstyle.1': 'マイペース',
  'playstyle.2': 'のんびり',
  'playstyle.3': 'ガチ勢',
  'playstyle.4': '初心者歓迎',
  'playstyle.5': 'わいわい',
  'playstyle.6': 'ランク2000+',
  'playstyle.7': 'ランク1000+',
  'playstyle.8': 'ランク500+',
  'playstyle.9': 'ランク250+',
  'playstyle.10': 'ランク100+',
  'playstyle.11': 'ランク20+',
  'playstyle.12': '毎日ログイン',
  'playstyle.13': '3日に1回ログイン',
  'playstyle.14': '朝に活動',
  'playstyle.15': '昼に活動',
  'playstyle.16': '夕方に活動',
  'playstyle.17': '深夜に活動',

  'form.label': 'サークルを探す',
  'form.searchBy': '検索方法',
  'form.trainerId': 'トレーナーID',
  'form.dailyPace': '1日の獲得ファン数',
  'form.remember': 'この端末にトレーナーIDを記憶する',
  'form.paceLabel': '1日のファン獲得数(百万単位)',
  'form.balanceScoring': 'バランス評価',
  'form.balanceStrength': ({ value }) => `バランスの強さ: ${value}`,
  'form.includeFull': '満員のサークルも含める',
  'form.hasDiscord': 'Discordあり',
  'form.joinStyle': '加入方法',
  'form.playstyle': 'サークル方針',
  'form.anyPlaystyle': '指定なし',
  'form.filterWarning':
    '加入方法やサークル方針で絞り込むと候補が少なくなるため、いつもより獲得ファン数のペースが離れたサークルが上位に出ることがあります。',
  'form.discordNote':
    'サークル紹介文にDiscordの記載があるかどうかで判定しています。Discordがあっても表示されないサークルや、期限切れのリンクが含まれる場合があります。',
  'form.find': 'サークルを探す',
  'form.searching': '検索中…',
  'form.trainerIdError': 'トレーナーIDを入力してください(9〜12桁の数字)。',
  'form.paceError': '1日のファン獲得数を百万単位で入力してください(例: 7.6)。',

  'status.loading': 'サークルを検索中…',
  'status.waitingTitle': 'サークルのデータを読み込み中です',
  'status.waitingBody': ({ wait }) =>
    `サーバーが初回のサークル取得を行っています(1分ほどかかります)。${wait}後に再試行します。`,
  'status.tryNow': '今すぐ再試行',
  'status.rateLimitedTitle': '検索回数が多すぎます',
  'status.rateLimitedWait': ({ wait }) => `${wait}待ってからもう一度検索してください。`,
  'status.rateLimitedDone': 'もう一度検索できます。',
  'status.tryAgain': '再試行',

  'error.generic.title': '問題が発生しました',
  'error.generic.detail': 'しばらくしてからもう一度お試しください。',
  'error.scanStillLoading.title': 'サークルのデータがまだ準備できていません',
  'error.scanStillLoading.detail':
    '初回の取得に通常より時間がかかっています。1分ほどしてからもう一度お試しください。',
  'error.network.title': 'サーバーに接続できません',
  'error.network.detail': '接続を確認して、もう一度お試しください。',
  'error.timeout.title': 'サーバーの応答に時間がかかっています',
  'error.invalidResponse.detail':
    'サーバーから予期しない応答がありました。しばらくしてからもう一度お試しください。',
  'error.humanCheck.title': '人間であることを確認できませんでした',
  'error.badSearch.title': '検索できませんでした',
  'error.trainerNotFound.title': 'トレーナーが見つかりません',
  'error.trainerNotFound.detail':
    'トレーナーIDを確認してください。IDは9〜12桁で、ゲーム内の「メニュー → プロフィール」に表示されます。また、uma.moeが追跡しているサークルに所属している必要があります。',
  'error.notEnoughData.title': 'ファン数のデータがまだ足りません',
  'error.notEnoughData.detail':
    'ペースを計算するには、uma.moeに今月3日分以上のファン数データが必要です。数日後にもう一度お試しいただくか、1日の獲得ファン数を入力して検索してください。',
  'error.umamoe.title': 'uma.moeに接続できません',
  'error.umamoe.detail':
    'サーバーがuma.moeからデータを取得できませんでした。しばらくしてからもう一度お試しください。',

  'hint.discord': '「Discordあり」をオフにする。',
  'hint.playstyle': 'サークル方針を「指定なし」に戻す。',
  'hint.closed': '加入方法で「募集停止」を選ぶと、現在募集していないサークルも表示されます。',
  'hint.balance': 'バランスの強さを下げるか、バランス評価をオフにする。',
  'notice.filtered':
    '加入方法やサークル方針で絞り込んでいるため、いつもより獲得ファン数のペースが離れたサークルが上位に出ることがあります。',
  'notice.pending': ({ n }) =>
    `あと${n}件のサークルを読み込み中です。1分ほどしてからもう一度検索すると、完全な結果が表示されます。`,
  'notice.lastMonth': ({ month }) =>
    `新しい月が始まったばかりのため、この結果は先月(${month})のデータに基づいています。`,
  'notice.blended':
    '新しい月が始まったばかりのため、この結果は今月と先月のデータを組み合わせています。先月のデータの比重は日ごとに小さくなります。',

  'results.heading': '検索結果',
  'results.topPicks': 'おすすめ',
  'results.allFull':
    '条件に合うサークルはすべて満員です。下の表には表示されていますが、加入できる枠はありません。',
  'results.none': '条件に合うサークルが見つかりませんでした。',
  'results.tryThese': '次を試してみてください:',
  'results.fullIncluded':
    '満員(30人中30人)のサークルは、加入できる枠がないため、おすすめには含まれず表のみに表示されます。',
  'results.fullExcluded':
    '満員(30人中30人)のサークルは、加入できる枠がないため、結果に含まれません。',
  'results.ownCompared': 'あなたのサークルはここには表示されず、下でおすすめ1位と比較しています。',
  'results.ownSearch':
    'あなたのサークルはここには表示されません。トレーナーIDで検索すると比較できます。',
  'results.whyTier': 'なぜ上位ティアのサークルが出ないの?',
  'results.whyTierBody':
    'サークルのティアは合計ファン数で決まり、上位メンバーの獲得数に大きく左右されます。このツールはサークルの典型的なメンバーを基準にするため、あなたが真ん中あたりに入れるサークルが選ばれます。上位ティアのサークルの典型的なメンバーよりあなたのペースがかなり低い場合は、1〜2ティア下のサークルが最適になることがあります。',
  'results.topN': ({ n }) => `上位${n}件`,

  'summary.pace': 'あなたのペース',
  'summary.analysed': '調べたサークル数',
  'summary.comfortable': 'ちょうどよいサークル',
  'summary.comfortableTitle': 'メンバーの上位25%〜60%あたりに入れるサークル',
  'summary.typical': '典型的な順位',
  'summary.typicalTitle': 'これらのサークルでのあなたの順位の中央値',
  'summary.updated': 'サークルのデータ更新:',

  'table.search': 'サークル名で検索',
  'table.searchPlaceholder': 'サークルを検索',
  'table.hideClosed': '募集停止のサークルを隠す',
  'table.showing': ({ shown, total }) => `${total}件中${shown}件のサークルを表示`,
  'table.region': '上位の結果',
  'table.empty': '該当するサークルがありません。',
  'table.col.pick': '#',
  'table.col.tier': 'ティア',
  'table.col.club': 'サークル',
  'table.col.rank': '順位',
  'table.col.lastMonth': '先月',
  'table.col.openSpots': '空き枠',
  'table.col.joinStyle': '加入方法',
  'table.col.median': '中央値/日',
  'table.col.place': 'あなたの順位',
  'table.col.balance': 'バランス',
  'table.col.passengers': 'おんぶ組',
  'table.col.top3': '上位3人の割合',
  'table.col.fit': '適合度',
  'table.col.score': 'スコア',

  'yourClub.heading': 'あなたのサークル',
  'yourClub.best': 'あなたのサークルは、どの検索結果よりもあなたに合っています。',
  'yourClub.rank': ({ rank }) => `あなたのサークルは検索結果の${rank}位に入ります。`,
  'yourClub.below': ({ n }) => `あなたのサークルは上位${n}件より下になります。`,
  'yourClub.typical': ({ own, pace }) =>
    `典型的なメンバーの獲得数は${own}で、あなたのペースは${pace}です。`,
  'yourClub.topPick': 'おすすめ1位',
  'yourClub.note': ({ name, score }) => `おすすめ1位: ${name}。${score}は低いほど良い値です。`,
  'yourClub.scoreWord': ({ balanced }) => (balanced ? 'スコア' : '適合度'),

  'tour.step': ({ n, total }) => `ステップ ${n} / ${total}`,
  'tour.skip': 'ツアーをスキップ',
  'tour.back': '戻る',
  'tour.next': '次へ',
  'tour.done': '完了',
  'tour.welcome.title': 'Uma Stable Matchへようこそ',
  'tour.welcome.body':
    '1日のファン獲得数がちょうど合い、メンバーが偏りなく貢献しているサークルを探します。簡単なツアーで使い方をご案内します。',
  'tour.searchBy.title': 'トレーナーIDまたはペースで検索',
  'tour.searchBy.body':
    '「トレーナーID」を選ぶとペースを自動で調べます。「1日の獲得ファン数」を選ぶと自分で入力できます。',
  'tour.field.title': '情報を入力',
  'tour.field.body':
    'トレーナーIDは、ゲーム内の「メニュー → プロフィール → トレーナーカード」に表示される9〜12桁の数字です。ペースで検索する場合は、1日のファン獲得数を百万単位で入力してください(例: 7.6)。',
  'tour.balance.title': 'バランス評価',
  'tour.balance.body':
    'オンにすると、一部のメンバーだけが支えているサークルの順位が下がります。スライダーで強さを調整できます。',
  'tour.filters.title': '検索を絞り込む',
  'tour.filters.body':
    '比較のために満員のサークルを含めたり、Discordの記載があるサークルだけに絞ったり、加入方法やサークル方針を1つ選んだりできます。絞り込むとペースの近いサークルが減るため、一致度が少し下がることがあります。',
  'tour.find.title': 'サークルを探す',
  'tour.find.body':
    '検索を実行します。検索条件はページのアドレスに保存されるので、ブックマークや共有ができます。',
  'tour.results.title': '結果の見方',
  'tour.results.body':
    '概要、おすすめをカードで、上位の結果を検索・並べ替えできる表で表示します。トレーナーIDで検索すると、今のサークルとの比較も表示されます。サークル名を選ぶとuma.moeのページが開き、サークルマスターに連絡できます。',
  'tour.howTo.title': '使い方',
  'tour.howTo.body': 'トレーナーIDやペースの調べ方など、手順はここでいつでも確認できます。',
  'tour.theme.title': 'ライト / ダーク',
  'tour.theme.body': '端末の設定に合わせるか、ライトまたはダークを選べます。',
  'tour.help.title': 'もう一度ツアーを見る',
  'tour.help.body': '疑問符のボタンを押すと、いつでもこのツアーをもう一度見られます。',

  'howTo.summary': '使い方',
  'howTo.step1':
    '<strong>トレーナーIDを調べる。</strong>ゲーム内で<strong>メニュー → プロフィール → トレーナーカード</strong>を開きます。プロフィールに表示されている9〜12桁の数字がトレーナーIDです。',
  'howTo.step2':
    '<strong>IDで検索する。</strong><strong>トレーナーID</strong>に入力して<strong>サークルを探す</strong>を押します。<strong>この端末にトレーナーIDを記憶する</strong>にチェックすると、次回から自動で入力されます。',
  'howTo.step3': ({ rankings }) =>
    `<strong>ペースで検索する。</strong><strong>1日の獲得ファン数</strong>に切り替えて、1日に獲得するファン数を百万単位で入力します(<strong>7.6</strong>なら1日760万)。調べるには、<a href="${rankings}" target="_blank" rel="noopener">uma.moeのランキング</a>でトレーナー名またはIDを検索し、1日平均のファン獲得数を確認してください。`,
  'howTo.step4':
    '<strong>必要に応じて調整する。</strong><strong>バランス評価</strong>は、一部のメンバーだけが支えているサークルの順位を下げます。<strong>満員のサークルも含める</strong>は、空きのないサークルを比較用に表へ追加します。<strong>加入方法</strong>(現在募集していないサークルなら<strong>募集停止</strong>など)や<strong>サークル方針</strong>を1つ選ぶと、そのサークルだけが表示されます。もう一度押すか<strong>指定なし</strong>を選ぶと元に戻ります。どちらも候補が少なくなるため、ペースの離れたサークルが上位に出ることがあります。<strong>Discordあり</strong>は、紹介文にDiscordの記載があるサークルに絞ります(あくまで目安です)。',
  'howTo.step5':
    '<strong>サークルを選ぶ。</strong>まずは<strong>おすすめ</strong>から見てみましょう。メンバーの真ん中あたりに入れるサークルが上位に出ます。<strong>上位の結果</strong>では検索や並べ替えができます。サークル名を選ぶとuma.moeのページが開き、サークルマスターに連絡できます。',
  'howTo.note':
    'ゲームのグローバル版(英語版)に対応しています。トレーナーIDが見つからない場合は、1日の獲得ファン数で検索してください。満員のサークルとあなた自身のサークルは結果に含まれません。自分のサークルとの比較は、トレーナーIDで検索すると表示されます。',

  'footer.tipsSummary': 'サークル移籍のヒント',
  'footer.tip14':
    '<strong>14日ルール:</strong>月間報酬を受け取るには、その月にサークルへ14日以上所属している必要があります。移籍は月初に済ませましょう。',
  'footer.tip24':
    '<strong>24時間のクールダウン:</strong>サークルを抜けると、24時間は別のサークルに加入できません。サークルマスターに除名してもらうとこの制限を避けられるという報告があります。',
  'footer.tipLock':
    '<strong>月初のロック:</strong>毎月1日のUTC 10:00〜14:59(日本時間19:00〜23:59)は、サークルの脱退、加入申請、招待ができません。',
  'footer.tipActivity':
    '<strong>先に活動状況を確認:</strong>申請する前に、uma.moeでメンバーの活動状況を確認しましょう。',
  'footer.tipsNote':
    'これらのルールは変更されることがあります。違いがあれば、ゲーム内で確認してください。',
  'footer.links': 'プロジェクトのリンク',
  'footer.report': '問題を報告',
  'footer.kofi': 'にんじんをおごる',
  'footer.carats': '無料ジュエル',
  'footer.support': '任意の支援はホスティング費用に充てられます。すべての機能は無料のままです。',
  'footer.unofficial':
    'Uma Stable Matchは<strong>非公式の非営利ファンツール</strong>です。uma.moeおよび株式会社Cygamesとは提携しておらず、公認・後援も受けていません。',
  'footer.data':
    'サークルとトレーナーのデータは<a href="https://uma.moe" target="_blank" rel="noopener noreferrer">uma.moe</a>から取得しており、遅れや欠けがある場合があります。おすすめは判断の目安となる推定であり、特定のプレイヤーやサークルを評価するものではありません。',
  'footer.trademark':
    '<em>ウマ娘 プリティーダービー</em>の名称、キャラクター、アートワーク、商標は株式会社Cygamesに帰属します。サークルのティアアイコンは© Cygames, Inc.のもので、Cygamesの<a href="https://umamusume.com/fan-createdguide" target="_blank" rel="noopener noreferrer">二次創作ガイドライン</a>に沿って非営利で使用しています。',
};
