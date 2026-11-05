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

  'nav.releases': '更新情報',

  'releases.title': '更新情報',
  'releases.intro': 'Uma Stable Matchのアップデート履歴です(新しい順)。',
  'releases.close': '閉じる',
  'releases.current': ({ build }) => `ビルド ${build}`,
  'releases.latest': '最新',
  'releases.kind.update': 'アップデート',
  'releases.kind.hotfix': '緊急修正',
  'releases.change.new': '新機能',
  'releases.change.improved': '改善',
  'releases.change.fixed': '修正',

  'releases.earlyChart.title': '月初めにサークルのグラフが表示されない問題を修正',
  'releases.earlyChart.body':
    '月初めにサークルページのグラフに「データが足りない」と表示されなくなり、今月のデータがそろうまでは先月のグラフを表示します。',
  'releases.clubPage.title': 'サークルでのあなたの位置を確認',
  'releases.clubPage.body':
    '各サークルの「メンバーと比べる」を選ぶと、メンバー内でのあなたの順位と、今月の各メンバーの1日のファン獲得数の推移をグラフで確認できます。',
  'releases.earlyMonth.title': '月初めの結果が安定',
  'releases.earlyMonth.body':
    '月初めは先月のペースを取り入れ、今月のデータがそろうまで結果の変動を抑えます。',
  'releases.yourClub.title': '今のサークルを比較',
  'releases.yourClub.body':
    'トレーナーIDで検索すると、今のサークルとおすすめの1位を比較できます。中央値とあなたのペース、メンバー内での順位、適合度を確認できます。',
  'releases.fullClubs.title': '満員のサークルも検索',
  'releases.fullClubs.body':
    '空きのないサークルを「満員」のバッジ付きで表示し、比較に使えます。おすすめには含まれません。',
  'releases.filters.title': 'サークル方針と加入方法で絞り込み',
  'releases.filters.body':
    '希望のサークル方針と、自由加入・承認制・募集停止の加入方法で結果を絞り込めます。「募集停止のサークルも含める」に代わる項目です。',
  'releases.movedClubs.title': '修正: 移籍後のトレーナーID検索',
  'releases.movedClubs.body':
    'サークルを移籍していても、トレーナーIDで検索して「見つかりません」と表示されることはなくなりました。',

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
  'club.seeMembers': 'メンバーを見る',
  'club.seeFit': 'メンバーと比べる',
  'club.seeFitIn': ({ name }) => `メンバーと比べる(${name})`,
  'club.seeMembersOf': ({ name }) => `メンバーを見る(${name})`,
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
  'table.col.link': 'サークルページ',

  'yourClub.heading': 'あなたのサークル',
  'yourClub.best': 'あなたのサークルは、どの検索結果よりもあなたに合っています。',
  'yourClub.rank': ({ rank }) => `あなたのサークルは検索結果の${rank}位に入ります。`,
  'yourClub.below': ({ n }) => `あなたのサークルは上位${n}件より下になります。`,
  'yourClub.typical': ({ own, pace }) =>
    `典型的なメンバーの獲得数は${own}で、あなたのペースは${pace}です。`,
  'yourClub.topPick': 'おすすめ1位',
  'yourClub.gap': 'あなたとの差',

  'clubPage.back': '検索に戻る',
  'clubPage.introYou':
    '加入を申請する前に、このサークルでのあなたの位置を確認できます。メンバー内での順位と、今月の各メンバーの1日のファン獲得数の推移を表示します。',
  'clubPage.introClub':
    'このサークルのメンバーと、今月の各メンバーの1日のファン獲得数の推移です。トレーナーIDで検索すると、メンバー内でのあなたの順位も確認できます。',
  'clubPage.title': ({ name }) => `${name} · Uma Stable Match`,
  'clubPage.loading': 'サークルを読み込んでいます…',
  'clubPage.median': '1日の中央値',
  'clubPage.yourPace': 'あなたのペース',
  'clubPage.youdBe': 'あなたの順位',
  'clubPage.inClub': 'あなたが所属しているサークルです。新しく加入した場合の順位を表示しています。',
  'clubPage.members': 'メンバー',
  'clubPage.membersNote': ({ n }) => `現在のメンバー${n}人(ペースの速い順)。`,
  'clubPage.data': ({ month, updated }) => `${month}のデータ(更新:${updated})。`,
  'clubPage.gap': ({ gap }) => `あなたと${gap}`,
  'clubPage.you': 'あなた',
  'clubPage.new': '加入したばかり',
  'clubPage.unknown': '名前不明のトレーナー',
  'clubPage.pickHint': 'メンバーを選ぶと、グラフでその線が強調表示されます。',
  'clubPage.notLoaded.title': 'このサークルはまだ読み込まれていません',
  'clubPage.notLoaded.detail':
    'サークルは検索で必要になったときに読み込まれます。時間をおいて再度お試しいただくか、検索に戻ってください。',

  'chart.heading': '今月のペース',
  'chart.show': 'グラフの表示',
  'chart.daily': '1日の獲得数',
  'chart.month': '今月の獲得数',
  'chart.labelDaily':
    '今月の各メンバーの1日のファン獲得数を、サークルの中央値とあなたの線と合わせて示す折れ線グラフです。矢印キーで日ごとの値を確認できます。',
  'chart.labelMonth':
    '今月の各メンバーのファン獲得数の累計を、サークルの中央値とあなたの線と合わせて示す折れ線グラフです。矢印キーで日ごとの値を確認できます。',
  'chart.members': 'メンバー',
  'chart.median': 'サークルの中央値',
  'chart.you': 'あなた',
  'chart.day': ({ n }) => `${n}日目`,
  'chart.range': ({ low, high }) => `${low}〜${high}`,
  'chart.empty': 'グラフを表示できるだけのデータがまだありません。',

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

  'nav.pages': 'ページ',
  'nav.home': 'ホーム',
  'nav.finder': 'サークル検索',
  'wishlist.title': 'ウィッシュリスト · Uma Stable Match',
  'wishlist.link': 'ウィッシュリスト',
  'wishlist.heading': '次に作るものを決める手伝いをしてください',
  'wishlist.intro':
    '作ってみたいアイデアをいくつか並べました。実際に使いたいと思うものに投票してください。',
  'wishlist.polls': '投票',
  'wishlist.poll.dashboard': 'リーダー向け応募ダッシュボードの投票',
  'wishlist.poll.discord': 'Discord Botによる応募連携の投票',
  'wishlist.poll.jpTrainers': '日本版のトレーナーとサークルの投票',
  'wishlist.aboutLabel': '投票について',
  'wishlist.about':
    'これらの投票は<a href="https://strawpoll.com" target="_blank" rel="noopener noreferrer">StrawPoll</a>が運営しており、Uma Stable Matchではありません。あなたの投票はStrawPollが<a href="https://strawpoll.com/privacy" target="_blank" rel="noopener noreferrer">プライバシーポリシー</a>に基づいて保存・管理します。',

  'club.applies': 'かんたん応募',
  'club.appliesTitle': 'Uma Stable Matchからこのサークルに直接応募できます',

  'applications.link': 'サークル申請',
  'applications.title': 'サークル申請 · Uma Stable Match',
  'applications.heading': 'どちらに当てはまりますか？',
  'applications.intro':
    'サークルマスターから応募者まで、次に進むべき一手がひと目で分かるように。サークル運営や応募状況を、一箇所でまとめて確認できます。',
  'applications.requirements':
    '<ul><li>サークルマスターはDiscord設定と運営状況をまとめて管理できます。</li><li>トレーナーは応募済みのサークルを一覧で確認し、進行状況を把握できます。</li></ul>',
  'applications.privacy': ({ href }: { href: string }) =>
    `Discordへのサインインは、サークル申請の管理のためだけに使われます。<a href="${href}">プライバシー情報</a>をご確認ください。`,
  'applications.session.loading': 'Discordセッションを確認しています…',
  'applications.session.signedOut':
    '以下から役割を選んでください。Discordでサインインしてサークルを管理できます。',

  'applications.session.signIn': 'Discordでサインイン',
  'applications.session.signedIn': 'Discordセッションは有効です。',

  'applications.session.signOut': 'サインアウト',
  'applications.session.error':
    'Discordセッションを確認できませんでした。しばらくしてからもう一度お試しください。',
  'applications.role.heading': 'どちらに当てはまりますか？',
  'applications.role.leader.title': 'サークルマスターです',
  'applications.role.leader.description':
    'ボットの状態を確認し、応募状況を見て、サークルのDiscord申請を管理します。',

  'applications.role.trainer.title': 'トレーナーです',
  'applications.role.trainer.description':
    '応募したサークルを一覧で確認し、応募履歴をまとめて見られます。',
  'applications.error.generic': 'サークル申請の準備中に問題が発生しました。',
  'applications.error.wrong_server': 'このDiscordサーバーはこのサークル向けに設定されていません。',
  'applications.error.not_manager':
    'ボットを追加するには、サークルのDiscordサーバーで「サーバー管理」または「管理者」の権限が必要です。',
  'applications.error.not_leader': 'ボットを追加できるのはサークルマスターだけです。',
  'applications.error.no_server': 'このサークルにはまだDiscordサーバーが接続されていません。',
  'applications.error.denied': 'Discordサインインが拒否されました。もう一度お試しください。',
  'applications.error.invalid_state':
    'Discordサインインの状態が無効になりました。もう一度お試しください。',
  'applications.error.discord_error':
    'Discordから予期しないエラーが返ってきました。もう一度お試しください。',
  'applications.error.unavailable':
    'サークル申請の設定は一時的に利用できません。しばらくしてからもう一度お試しください。',

  'footer.tipsSummary': 'サークル移籍のヒント',
  'footer.tip14':
    '<strong>14日ルール:</strong>月間報酬を受け取るには、その月にサークルへ14日以上所属している必要があります。移籍は月初に済ませましょう。',

  'footer.tip24':
    '<strong>24時間のクールダウン:</strong>サークルを抜けると、24時間は別のサークルに加入できません。代わりにサークルマスターに除名してもらうことで、この制限を避ける人もいます。',

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

  'trainer.title': '自分の申請 · Uma Stable Match',
  'trainer.eyebrow': 'サークル申請',
  'trainer.heading': '自分の申請',
  'trainer.lede':
    '申請したサークルと、それぞれの申請の状況です。申請は送信から1か月で期限切れになります。',
  'trainer.table.heading': '有効な申請',
  'trainer.table.empty': '有効な申請はありません',
  'trainer.table.emptyHint':
    'サークル検索からサークルに申請すると、その申請が1か月間ここに表示されます。',
  'trainer.table.club': 'サークル',
  'trainer.table.trainerId': 'トレーナーID',
  'trainer.table.expires': '期限',
  'trainer.table.ticket': 'Discordで開く',
  'trainer.table.ticketFor': ({ name }: { name: string }) => `${name}の申請をDiscordで開く`,
  'trainer.table.noTicket': 'チケット終了',

  'leader.title': 'サークル申請 · Uma Stable Match',
  'leader.eyebrow': 'サークル申請',
  'leader.heading': 'あなたのサークル',
  'leader.loading': '読み込み中…',
  'leader.signIn':
    'サインインの有効期限が切れました。このサークルを管理するにはDiscordでサインインしてください。',
  'leader.forbidden':
    'このサークルを管理できるのはサークルマスターだけです。設定で連携しているトレーナーを確認してください。',
  'leader.loadError': 'サークルを読み込めませんでした。しばらくしてからもう一度お試しください。',
  'leader.linked': 'Botがサーバーに追加されました。',
  'leader.state.off': '未設定',
  'leader.state.open': '申請受付中',
  'leader.state.paused': '一時停止中',
  'leader.state.problem': '要対応',
  'leader.callout.problem':
    '現在、プレイヤーは申請できません。「Botの状態」に表示されている問題を解決してください。',
  'leader.callout.paused':
    '申請は一時停止中です。プレイヤーにはこのサークルの申請ボタンが表示されません。',
  'leader.callout.resume': '再開する',
  'leader.setup.eyebrow': 'Discordでのサークル申請',
  'leader.setup.heading': ({ name }: { name: string }) =>
    `${name}にワンクリックで申請できるようにする`,
  'leader.setup.yourClub': 'あなたのサークル',
  'leader.setup.lede':
    'ここでサークルを見つけたプレイヤーがすぐに申請できます。申請はあなたのDiscordサーバーで確認します。',
  'leader.flow.applyTitle': 'プレイヤーが申請',
  'leader.flow.applyText': 'サークルのページや検索結果から申請します。',
  'leader.flow.ticketTitle': 'Botがチケットを作成',
  'leader.flow.ticketText': 'サーバー内に、ペースとファン数を載せた専用チャンネルを作ります。',
  'leader.flow.decideTitle': 'あなたが判断',
  'leader.flow.decideText': 'Discordで判断し、すべての申請をこのページで確認できます。',
  'leader.steps.heading': '設定手順',
  'leader.steps.done': '完了',
  'leader.steps.todo': '未完了',
  'leader.steps.account': 'ウマ娘のアカウントを連携する',
  'leader.steps.accountHow':
    '設定でトレーナーIDを連携してください。これでどのサークルのマスターかがわかります。ボットを追加できるのはサークルマスターだけです。',
  'leader.steps.accountDone': ({ id }: { id: string }) => `トレーナー ${id} を連携済みです。`,
  'leader.steps.notLeader':
    'このトレーナーはサークルマスターではありません。ボットを追加できるのはサークルマスターだけです。',
  'leader.steps.openSettings': '設定を開く',
  'leader.steps.invite': 'サークル説明にDiscordサーバーの招待リンクを載せる',
  'leader.steps.inviteHow':
    'これでサーバーがあなたのサークルのものだとわかります。ゲーム内でサークル説明を編集し、discord.gg/yourcode のような招待リンクを貼り付けてください。サーバーへの招待ならどれでも使え、設定後は期限が切れても構いません。新しい説明がuma.moeに反映されるまで時間がかかることがあります。',
  'leader.steps.inviteFound': 'サークル説明にサーバーへの招待が見つかりました。',
  'leader.steps.inviteMissing': 'サークル説明に、1つのサーバーへの有効な招待がまだ見つかりません。',
  'leader.steps.checking': 'サークル説明を確認中…',
  'leader.steps.checkAgain': 'もう一度確認する',
  'leader.steps.permissions':
    'そのサーバーで「サーバー管理」または「管理者」の権限があることを確認する',
  'leader.steps.permissionsHow':
    'Botを追加できるのは、どちらかの権限を持つ人だけです。サーバーの所有者は両方を持っています。持っていない場合は、サーバーの管理者に次の手順を依頼してください。',
  'leader.steps.addBot': 'Botを追加する',
  'leader.steps.addBotHow':
    'サーバーが選択された状態でDiscordが開きます。Botを追加すると、このダッシュボードに戻ります。',
  'leader.addBot': 'Botをサーバーに追加',
  'leader.status.heading': 'Botの状態',
  'leader.status.inServer': 'Botはサーバーにいます',
  'leader.status.problem': ({ problem }: { problem: string }) => `修正が必要です: ${problem}`,
  'leader.status.open': '申請受付中',
  'leader.status.paused': '申請は一時停止中です',
  'leader.settings.heading': '設定',
  'leader.settings.open': '申請を受け付ける',
  'leader.settings.openHint': 'オフにすると、このサークルの申請ボタンが非表示になります。',
  'leader.settings.autoJoin': '申請者をサーバーに追加する',
  'leader.settings.autoJoinHint': '申請者は申請時にサーバーへ参加します。',
  'leader.settings.save': '保存',
  'leader.settings.saving': '保存中…',
  'leader.settings.saved': '保存しました。',
  'leader.settings.saveError': '変更を保存できませんでした。',
  'leader.remove.heading': 'Botを外す',
  'leader.remove.hint':
    'このサークルの申請を止め、サーバーとの連携を削除します。そのサーバーを使うサークルがほかになければ、Botはサーバーから退出します。',
  'leader.remove.button': 'このサークルからBotを外す',
  'leader.remove.confirm': 'このサークルからBotを外しますか？後からもう一度追加できます。',
  'leader.remove.yes': '外す',
  'leader.remove.cancel': 'キャンセル',
  'leader.remove.error': '今はBotを外せませんでした。',
  'leader.table.heading': '申請',
  'leader.table.empty': 'まだ申請はありません',
  'leader.table.emptyHint':
    'プレイヤーが申請すると、こことサーバー内のチケットチャンネルに表示されます。',
  'leader.table.trainer': 'トレーナー',
  'leader.table.submitted': '申請日',
  'leader.table.pace': 'ペース',
  'leader.table.fans': '今月のファン数',
  'leader.table.currentClub': '現在のサークル',
  'leader.table.noClub': 'なし',
  'leader.table.status': '状態',
  'leader.table.actions': '操作',
  'leader.table.view': '表示',
  'leader.table.viewFor': ({ name }: { name: string }) => `${name}の申請を表示`,
  'leader.filter.label': '状態で絞り込む',
  'leader.filter.all': 'すべて',
  'leader.filter.pending': '保留中',
  'leader.filter.accepted': '承認済み',
  'leader.filter.declined': '不承認',
  'leader.applicationStatus.pending': '保留中',
  'leader.applicationStatus.accepted': '承認済み',
  'leader.applicationStatus.declined': '不承認',
  'leader.details.heading': ({ name }: { name: string }) => `${name}の申請`,
  'leader.details.close': '閉じる',
  'leader.details.intro': 'Botがサーバーに投稿した内容です。',
  'leader.details.embedTitle': ({ name }: { name: string }) => `${name}への新しい申請`,
  'leader.details.embedUnnamed': '新しい申請',
  'leader.details.trainerId': 'トレーナーID',
  'leader.details.discord': 'Discord',
  'leader.details.submitted': ({ date }: { date: string }) => `申請日: ${date}`,
  'leader.details.openTicket': 'Discordでチケットを開く',
  'leader.details.ticketGone': 'チケットチャンネルは閉じられました。',

  'account.label': 'アカウント',
  'account.signIn': 'サインイン',
  'account.unnamed': 'Discordアカウント',
  'account.signedInAs': ({ name }: { name: string }) => `${name}としてサインイン中`,
  'account.settings': '設定',
  'account.signOut': 'サインアウト',
  'account.signOutError': 'サインアウトできませんでした。しばらくしてからもう一度お試しください。',

  'settings.title': '設定 · Uma Stable Match',
  'settings.heading': '設定',
  'settings.lede': 'Uma Stable Matchのアカウントを管理します。',
  'settings.link.heading': 'ウマ娘のアカウントを連携',
  'settings.link.intro':
    'ゲーム内のプロフィールコメントに短いキーを追加して、そのトレーナーIDが自分のものだと確認します。',
  'settings.link.label': 'トレーナーID',
  'settings.link.hint': 'トレーナープロフィールに表示される9〜12桁の数字です。',
  'settings.link.invalid': '9〜12桁のトレーナーIDを入力してください。',
  'settings.link.submit': 'アカウントを連携',
  'settings.link.keyHeading': ({ id }: { id: string }) =>
    `トレーナー${id}のプロフィールコメントにこのキーを追加してください`,
  'settings.link.keySteps':
    'ゲーム内でトレーナープロフィールを開き、コメントを編集してキーをどこかに追加します。その後「確認」を選んでください。uma.moeに新しいコメントが反映されるまで時間がかかることがあります。キーは24時間有効です。',
  'settings.link.copy': 'キーをコピー',
  'settings.link.copied': 'コピーしました',
  'settings.link.check': '確認',
  'settings.link.checking': '確認中…',
  'settings.link.cancel': 'キャンセル',
  'settings.link.verifying': '確認中',
  'settings.link.verifyingHint':
    'uma.moeでキーを今すぐ確認し、その後5分間は1分ごとに確認します。このページを離れても確認は続きます。',
  'settings.link.missing':
    '5分たってもコメントにキーが見つかりませんでした。uma.moeに表示されているか確かめてから、もう一度「確認」を選んでください。',
  'settings.link.linked': ({ name }: { name: string }) =>
    `${name}を連携しました。コメントからキーを削除しても大丈夫です。`,
  'settings.link.notFound': 'uma.moeでそのトレーナーが見つかりませんでした。',
  'settings.link.already': 'そのトレーナーはすでに連携されています。',
  'settings.link.tooMany': ({ n }: { n: number }) =>
    n === 1
      ? '連携できるトレーナーは1人だけです。先に連携を解除してください。'
      : `連携できるトレーナーは${n}人までです。先にどれかの連携を解除してください。`,
  'settings.link.full':
    'トレーナーを連携済みです。別のトレーナーを連携するには、先に連携を解除してください。',
  'settings.link.tooManyTries': '試行回数が多すぎます。しばらくしてからもう一度お試しください。',
  'settings.link.expired':
    'キーの有効期限が切れました。最初からやり直して新しいキーを取得してください。',
  'settings.link.error': '問題が発生しました。しばらくしてからもう一度お試しください。',
  'settings.link.linkedHeading': '連携中のトレーナー',
  'settings.link.none': 'まだ連携しているトレーナーはいません。',
  'settings.link.unlink': '連携解除',
  'settings.link.unlinkFor': ({ id }: { id: string }) => `トレーナー${id}の連携を解除`,
  'settings.link.loadError': '連携中のトレーナーを読み込めませんでした。',
  'settings.link.unavailable': '現在トレーナーの連携は利用できません。',
  'settings.session.heading': 'サインアウト',
  'settings.session.text': 'この端末でのセッションを終了します。',
  'settings.signOut': 'サインアウト',
  'settings.delete.heading': 'アカウントを削除',
  'settings.delete.text':
    '連携中のトレーナーと確認待ちのキーを削除し、サインアウトします。Botを追加したサークルはそのまま残ります。',
  'settings.delete.button': 'アカウントを削除',
  'settings.delete.confirm':
    'アカウントを削除しますか？連携中のトレーナーは削除され、サインアウトします。元に戻すことはできません。',
  'settings.delete.yes': '削除',
  'settings.delete.cancel': 'キャンセル',
  'settings.delete.error':
    '今はアカウントを削除できませんでした。しばらくしてからもう一度お試しください。',
};
