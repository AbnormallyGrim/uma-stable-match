/**
 * English messages, and the shape every language follows.
 *
 * A message is a string, or a function of named values for anything with numbers or plurals. The
 * few that hold markup (`howTo.*`, `footer.*`) are first-party text shown with `{@html}`.
 */

const plural = (n: number, one: string, other: string) => (n === 1 ? one : other);

export const en = {
  'app.name': 'Uma Stable Match',
  'app.heading': 'Uma Stable Match club finder',
  'app.found': ({ n }: { n: number }) => `Found ${n} ${plural(n, 'club', 'clubs')}.`,

  'nav.label': 'Main',
  'nav.tour': 'Take the tour',

  'language.label': 'Language',
  'language.en': 'English',
  'language.ja': '日本語',

  'theme.label': 'Theme',
  'theme.system': 'System',
  'theme.light': 'Light',
  'theme.dark': 'Dark',

  'time.justNow': 'just now',
  'time.unknown': 'unknown',
  'time.seconds': ({ n }: { n: number }) => `${n} ${plural(n, 'second', 'seconds')}`,

  'format.perDay': ({ value }: { value: string }) => `${value}/day`,
  'format.place': ({ place, size }: { place: number; size: number }) => `${place} of ${size}`,
  'format.topShare': ({ percent }: { percent: number }) => `Top ${percent}%`,
  'month.label': ({ year, month }: { year: string; month: number }) =>
    `${
      [
        'January',
        'February',
        'March',
        'April',
        'May',
        'June',
        'July',
        'August',
        'September',
        'October',
        'November',
        'December',
      ][month - 1]
    } ${year}`,

  'trend.unknown': 'Rank unknown',
  'trend.new': 'new',
  'trend.notRanked': 'Not ranked last month',
  'trend.up': ({ n }: { n: number }) => `Up ${n} ${plural(n, 'place', 'places')} since last month`,
  'trend.down': ({ n }: { n: number }) =>
    `Down ${n} ${plural(n, 'place', 'places')} since last month`,
  'trend.same': 'Same rank as last month',

  'club.full': 'Full',
  'club.openSpots': ({ n }: { n: number }) => `${n} open ${plural(n, 'spot', 'spots')}`,
  'club.rank': ({ rank }: { rank: number }) => `Rank ${rank}`,
  'club.pick': ({ n }: { n: number }) => `Pick ${n}`,
  'club.discord': 'Discord',
  'club.discordTitle': 'The description mentions Discord',
  'club.youdBe': "You'd be",
  'club.median': 'Median',
  'club.balance': 'Balance',
  'club.passengers': 'Passengers',
  'club.top3Share': 'Top 3 share',
  'club.score': 'Score',
  'club.fit': 'Fit',
  'tier.label': ({ tier }: { tier: string }) => `Club tier ${tier}`,
  'tier.unknown': 'Club tier unknown',

  'join.open': 'Open',
  'join.approval': 'Approval',
  'join.closed': 'Closed',

  'playstyle.1': 'You Do You',
  'playstyle.2': 'Laid-back',
  'playstyle.3': 'Going for Gold',
  'playstyle.4': 'Beginners Welcome',
  'playstyle.5': "Let's Party!",
  'playstyle.6': 'Rank 2000+',
  'playstyle.7': 'Rank 1000+',
  'playstyle.8': 'Rank 500+',
  'playstyle.9': 'Rank 250+',
  'playstyle.10': 'Rank 100+',
  'playstyle.11': 'Rank 20+',
  'playstyle.12': 'Log in Daily',
  'playstyle.13': 'Log in Every 3 Days',
  'playstyle.14': 'Active in the Morning',
  'playstyle.15': 'Active in the Afternoon',
  'playstyle.16': 'Active in the Evening',
  'playstyle.17': 'Active at Night',

  'form.label': 'Find clubs',
  'form.searchBy': 'Search by',
  'form.trainerId': 'Trainer ID',
  'form.dailyPace': 'Daily pace',
  'form.remember': 'Remember my trainer ID on this device',
  'form.paceLabel': 'Daily fan gain (millions)',
  'form.balanceScoring': 'Balance scoring',
  'form.balanceStrength': ({ value }: { value: string }) => `Balance strength: ${value}`,
  'form.includeFull': 'Include full clubs',
  'form.hasDiscord': 'Has Discord',
  'form.joinStyle': 'Join style',
  'form.playstyle': 'Playstyle',
  'form.anyPlaystyle': 'Any playstyle',
  'form.filterWarning':
    'Filtering by join style or playstyle leaves fewer clubs to choose from, so your best matches may be further from your pace than usual.',
  'form.discordNote':
    "Based on whether the club's description mentions Discord. Some clubs with a Discord won't show up, and some links may have expired.",
  'form.find': 'Find clubs',
  'form.searching': 'Searching…',
  'form.trainerIdError': 'Enter your trainer ID: 9 to 12 digits.',
  'form.paceError': 'Enter your daily fan gain in millions, such as 7.6.',

  'status.loading': 'Finding clubs…',
  'status.waitingTitle': 'Club data is still loading',
  'status.waitingBody': ({ wait }: { wait: string }) =>
    `The server is scanning clubs for the first time, which takes about a minute. Trying again in ${wait}.`,
  'status.tryNow': 'Try now',
  'status.rateLimitedTitle': 'Too many searches',
  'status.rateLimitedWait': ({ wait }: { wait: string }) =>
    `Please wait ${wait} before searching again.`,
  'status.rateLimitedDone': 'You can search again now.',
  'status.tryAgain': 'Try again',

  'error.generic.title': 'Something went wrong',
  'error.generic.detail': 'Try again in a moment.',
  'error.scanStillLoading.title': "Club data still isn't ready",
  'error.scanStillLoading.detail':
    'The first scan is taking longer than usual. Try again in a minute.',
  'error.network.title': "Couldn't reach the server",
  'error.network.detail': 'Check your connection and try again.',
  'error.timeout.title': 'The server took too long to answer',
  'error.invalidResponse.detail': 'The server sent an unexpected response. Try again in a moment.',
  'error.humanCheck.title': "We couldn't confirm you're a person",
  'error.badSearch.title': "That search didn't work",
  'error.trainerNotFound.title': "We couldn't find that trainer",
  'error.trainerNotFound.detail':
    "Check the trainer ID. It's 9 to 12 digits, shown in-game under Menu → Profile. The trainer also needs to be in a club that uma.moe tracks.",
  'error.notEnoughData.title': 'Not enough fan data yet',
  'error.notEnoughData.detail':
    'uma.moe needs at least 3 days of fan data this month to work out your pace. Try again in a few days, or search by daily pace instead.',
  'error.umamoe.title': "Couldn't reach uma.moe",
  'error.umamoe.detail': "The server couldn't get your data from uma.moe. Try again in a moment.",

  'hint.discord': 'Turn off Has Discord.',
  'hint.playstyle': 'Set playstyle back to any.',
  'hint.closed': "Pick Closed under join style to see clubs that aren't recruiting right now.",
  'hint.balance': 'Lower the balance strength or turn balance scoring off.',
  'notice.filtered':
    'These results are filtered by join style or playstyle, so your best matches may be further from your pace than usual.',
  'notice.pending': ({ n }: { n: number }) =>
    `${n} more ${plural(n, 'club is', 'clubs are')} still loading. Search again in a minute for complete results.`,
  'notice.lastMonth': ({ month }: { month: string }) =>
    `These results are based on last month's data (${month}), because the new game month has only just started.`,

  'results.heading': 'Results',
  'results.topPicks': 'Top picks',
  'results.allFull':
    "Every matching club is full. They're listed below, but there's no spot to join.",
  'results.none': 'No clubs matched your search.',
  'results.tryThese': 'You could try:',
  'results.fullIncluded':
    "Full clubs (30 of 30 members) are in the table, but not the top picks, since there's no spot to join.",
  'results.fullExcluded':
    "Full clubs (30 of 30 members) aren't in the results, since there's no spot to join.",
  'results.ownCompared':
    "Your current club is never listed here: it's compared with your top pick below.",
  'results.ownSearch':
    'Your current club is never listed here: search by trainer ID to see how it compares.',
  'results.whyTier': 'Why not a higher-tier club?',
  'results.whyTierBody':
    "A club's tier comes from its total fans, which its fastest members drive. Matches are based on a club's typical member instead, so you land in clubs where you'd sit around the middle. If you're well below a high-tier club's typical member, your best fits may be a tier or two lower.",
  'results.topN': ({ n }: { n: number }) => `Top ${n} ${plural(n, 'result', 'results')}`,

  'summary.pace': 'Your pace',
  'summary.analysed': 'Clubs analysed',
  'summary.comfortable': 'Comfortable fits',
  'summary.comfortableTitle': "Clubs where you'd sit 25% to 60% of the way down the members",
  'summary.typical': 'Typical place',
  'summary.typicalTitle': 'Your middle place across these clubs',
  'summary.updated': 'Club data updated',

  'table.search': 'Search clubs by name',
  'table.searchPlaceholder': 'Search clubs',
  'table.hideClosed': 'Hide closed clubs',
  'table.showing': ({ shown, total }: { shown: number; total: number }) =>
    `Showing ${shown} of ${total} ${plural(total, 'club', 'clubs')}`,
  'table.region': 'Top results',
  'table.empty': 'No clubs match.',
  'table.col.pick': '#',
  'table.col.tier': 'Tier',
  'table.col.club': 'Club',
  'table.col.rank': 'Rank',
  'table.col.lastMonth': 'Last month',
  'table.col.openSpots': 'Open spots',
  'table.col.joinStyle': 'Join style',
  'table.col.median': 'Median/day',
  'table.col.place': "You'd be",
  'table.col.balance': 'Balance',
  'table.col.passengers': 'Passengers',
  'table.col.top3': 'Top 3 share',
  'table.col.fit': 'Fit',
  'table.col.score': 'Score',

  'yourClub.heading': 'Your club',
  'yourClub.best': 'Your club fits you better than any of your results.',
  'yourClub.rank': ({ rank }: { rank: number }) => `Your club would be #${rank} in your results.`,
  'yourClub.below': ({ n }: { n: number }) => `Your club would rank below your top ${n}.`,
  'yourClub.typical': ({ own, pace }: { own: string; pace: string }) =>
    `Its typical member gains ${own}, against your ${pace}.`,
  'yourClub.topPick': 'Top pick',
  'yourClub.note': ({ name, score }: { name: string; score: string }) =>
    `Top pick: ${name}. A lower ${score} is better.`,
  'yourClub.scoreWord': ({ balanced }: { balanced: boolean }): string =>
    balanced ? 'score' : 'fit',

  'tour.step': ({ n, total }: { n: number; total: number }) => `Step ${n} of ${total}`,
  'tour.skip': 'Skip tour',
  'tour.back': 'Back',
  'tour.next': 'Next',
  'tour.done': 'Done',
  'tour.welcome.title': 'Welcome to Uma Stable Match',
  'tour.welcome.body':
    'Find clubs where your daily fan gain fits and members pull their weight. This quick tour shows you around.',
  'tour.searchBy.title': 'Search by trainer ID or pace',
  'tour.searchBy.body':
    'Pick Trainer ID to look up your pace automatically, or Daily pace to type it in yourself.',
  'tour.field.title': 'Enter your details',
  'tour.field.body':
    'Your trainer ID is the 9 to 12 digit number on your profile, under Menu → Profile Trainer Card. For daily pace, enter the fans you gain a day in millions, such as 7.6.',
  'tour.balance.title': 'Balance scoring',
  'tour.balance.body':
    'Turn this on to rank clubs lower when a few members carry everyone else. The slider sets how strongly.',
  'tour.filters.title': 'Narrow your search',
  'tour.filters.body':
    'Include full clubs to compare against, keep only clubs that mention Discord, or pick one join style or playstyle. Filtering leaves fewer clubs near your pace, so matches can be a little looser.',
  'tour.find.title': 'Find clubs',
  'tour.find.body':
    'Run the search. It is saved in the page address, so you can bookmark or share it.',
  'tour.results.title': 'Read your results',
  'tour.results.body':
    "You'll get a summary, your top picks as cards, and the top results in a table you can search and sort. Searching by trainer ID also compares your current club. Select a club's name to open its uma.moe page and contact the leader.",
  'tour.howTo.title': 'How to use',
  'tour.howTo.body':
    'Step-by-step instructions, including where to find your trainer ID and pace, are here whenever you need them.',
  'tour.theme.title': 'Light or dark',
  'tour.theme.body': 'Follow your device, or pick the light or dark theme.',
  'tour.help.title': 'Take the tour again',
  'tour.help.body': 'Select the question mark to bring this tour back any time.',

  'howTo.summary': 'How to use',
  'howTo.step1':
    '<strong>Find your trainer ID.</strong> In the game, open <strong>Menu → Profile Trainer Card</strong>. Your trainer ID is the 9 to 12 digit number on your profile.',
  'howTo.step2':
    '<strong>Search with it.</strong> Enter it under <strong>Trainer ID</strong> and select <strong>Find clubs</strong>. Tick <strong>Remember my trainer ID on this device</strong> to have it filled in next time.',
  'howTo.step3': ({ rankings }: { rankings: string }) =>
    `<strong>Or search by your pace.</strong> Switch to <strong>Daily pace</strong> and enter how many fans you gain a day, in millions: <strong>7.6</strong> means 7.6 million a day. To find it, search for your trainer name or ID in <a href="${rankings}" target="_blank" rel="noopener">uma.moe's rankings</a> and use your average daily fan gain.`,
  'howTo.step4':
    "<strong>Fine-tune, if you like.</strong> <strong>Balance scoring</strong> ranks clubs where a few members carry everyone lower. <strong>Include full clubs</strong> adds clubs with no open spot to the table, to compare against. Pick one <strong>Join style</strong>, such as <strong>Closed</strong> for clubs that aren't recruiting right now, or one <strong>Playstyle</strong>, to see only those clubs. Pick it again, or choose <strong>Any playstyle</strong>, to go back. Either leaves fewer clubs to choose from, so your best matches may be further from your pace. <strong>Has Discord</strong> keeps clubs whose description mentions Discord, as a best guess.",
  'howTo.step5':
    "<strong>Pick a club.</strong> Start with the <strong>Top picks</strong>: the best fits are clubs where you'd sit around the middle of the members. Use <strong>Top results</strong> to search and sort more. Select a club's name to open its uma.moe page and contact the leader.",
  'howTo.note':
    "Works with the global (English) version of the game. If your trainer ID isn't found, search by your daily pace instead. Full clubs and your own club aren't in the results. Search by trainer ID to see how your club compares.",

  'footer.tipsSummary': 'Tips for switching clubs',
  'footer.tip14':
    '<strong>14-day rule:</strong> you need at least 14 days in a club during a month to receive its monthly reward, so switch early in the month.',
  'footer.tip24':
    "<strong>24-hour cooldown:</strong> after leaving a club you can't join another for 24 hours. Players report that being removed by the leader avoids this.",
  'footer.tipLock':
    "<strong>Month-start lock:</strong> from 10:00 to 14:59 UTC on the 1st, you can't leave, apply to or invite to clubs.",
  'footer.tipActivity':
    "<strong>Check activity first:</strong> review members' activity on uma.moe before applying.",
  'footer.tipsNote': "These rules can change, so check in-game if something doesn't match.",
  'footer.links': 'Project links',
  'footer.report': 'Report a problem',
  'footer.kofi': 'Buy me a carrot',
  'footer.carats': 'Free carats',
  'footer.support': 'Optional tips help cover hosting. Every feature stays free.',
  'footer.unofficial':
    "Uma Stable Match is an <strong>unofficial, non-commercial fan tool</strong>. It isn't affiliated with, endorsed by or sponsored by uma.moe or Cygames, Inc.",
  'footer.data':
    'Club and trainer data comes from <a href="https://uma.moe" target="_blank" rel="noopener noreferrer">uma.moe</a> and may be delayed or incomplete. Recommendations are estimates to help you decide, not judgements of any player or club.',
  'footer.trademark':
    '<em>Umamusume: Pretty Derby</em> and its names, characters, artwork and trademarks belong to Cygames, Inc. The club tier icons are © Cygames, Inc. and are used non-commercially in line with Cygames\' <a href="https://umamusume.com/fan-createdguide" target="_blank" rel="noopener noreferrer">Fan-Created Content Guidelines</a>.',
};

/** The messages every language provides. */
export type Catalog = typeof en;
