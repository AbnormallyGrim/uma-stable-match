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

  'nav.releases': "What's new",

  'releases.title': "What's new",
  'releases.intro': 'Every update to Uma Stable Match, newest first.',
  'releases.close': 'Close',
  'releases.current': ({ build }: { build: string }) => `Build ${build}`,
  'releases.latest': 'Latest',
  'releases.kind.update': 'Update',
  'releases.kind.hotfix': 'Hotfix',
  'releases.change.new': 'New',
  'releases.change.improved': 'Improved',
  'releases.change.fixed': 'Fixed',

  'releases.quickApply.title': 'Apply to clubs with Quick Apply',
  'releases.quickApply.body':
    'Sign in with Discord and apply to any club that has the bot with one click, then follow your applications under Applications.',
  'releases.leaderApplications.title': 'Take applications for your club',
  'releases.leaderApplications.body':
    'Club leaders can add the bot to their Discord server, set their requirements, and accept or decline each trainer with their fan pace and history in front of them.',
  'releases.leaderApplications.link': 'Open Applications',
  'releases.trainerLookup.title': "Fixed trainers not being found when they're between clubs",
  'releases.trainerLookup.body':
    "Searching by trainer ID now gives recommendations for trainers who aren't in a club right now, or have no fan gains yet this month.",
  'releases.earlyChart.title': 'Fixed the empty club chart early in the month',
  'releases.earlyChart.body':
    "The club page chart no longer says there are too few days of data at the start of the month, and shows last month's until this month has enough.",
  'releases.clubPage.title': 'See how you fit in a club',
  'releases.clubPage.body':
    "Select See how you fit on any club to see where you'd rank among its members and follow everyone's daily fan gain this month on a chart.",
  'releases.earlyMonth.title': 'Steadier results early in the month',
  'releases.earlyMonth.body':
    "Get steadier results early in the game month, with last month's pace blended in until this month has enough data.",
  'releases.yourClub.title': 'Compare your current club',
  'releases.yourClub.body':
    'Search by trainer ID to compare your current club with your top pick, including its median pace against yours, your place among its members and its fit.',
  'releases.fullClubs.title': 'Include full clubs',
  'releases.fullClubs.body':
    'Show clubs with no open spot, marked with a Full badge, to compare against. They stay out of your top picks.',
  'releases.filters.title': 'Filter by playstyle and join style',
  'releases.filters.body':
    'Narrow your results to the playstyles you want and to Open, Approval or Closed clubs, in place of Include closed clubs.',
  'releases.movedClubs.title': 'Fixed trainer ID search after changing clubs',
  'releases.movedClubs.body':
    "Searching by trainer ID no longer says your trainer wasn't found if you've changed clubs.",

  'language.switch': 'Switch to Japanese (日本語)',

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
  'club.seeMembers': 'See members',
  'club.seeFit': 'See how you fit',
  'club.seeFitIn': ({ name }: { name: string }) => `See how you fit in ${name}`,
  'club.seeMembersOf': ({ name }: { name: string }) => `See members of ${name}`,
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
  'notice.blended':
    "The game month has only just started, so these results combine this month's data with last month's. Last month's counts for less each day.",

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
  'table.col.link': 'Club page',

  'yourClub.heading': 'Your club',
  'yourClub.best': 'Your club fits you better than any of your results.',
  'yourClub.rank': ({ rank }: { rank: number }) => `Your club would be #${rank} in your results.`,
  'yourClub.below': ({ n }: { n: number }) => `Your club would rank below your top ${n}.`,
  'yourClub.typical': ({ own, pace }: { own: string; pace: string }) =>
    `Its typical member gains ${own}, against your ${pace}.`,
  'yourClub.topPick': 'Top pick',
  'yourClub.gap': 'Vs your pace',

  'clubPage.back': 'Back to search',
  'clubPage.introYou':
    "See how you'd fit in this club before you apply: where you'd rank among its members, and how everyone's daily fan gain has gone this month.",
  'clubPage.introClub':
    "This club's members and how everyone's daily fan gain has gone this month. Search by trainer ID to see where you'd rank among them.",
  'clubPage.title': ({ name }: { name: string }) => `${name} · Uma Stable Match`,
  'clubPage.loading': 'Loading the club…',
  'clubPage.median': 'Median/day',
  'clubPage.yourPace': 'Your pace',
  'clubPage.youdBe': "You'd be",
  'clubPage.inClub': "This is your club. You're placed as if you were joining it.",
  'clubPage.members': 'Members',
  'clubPage.membersNote': ({ n }: { n: number }) =>
    `${n} current ${plural(n, 'member', 'members')}, fastest first.`,
  'clubPage.data': ({ month, updated }: { month: string; updated: string }) =>
    `Data for ${month}, updated ${updated}.`,
  'clubPage.gap': ({ gap }: { gap: string }) => `${gap} vs you`,
  'clubPage.you': 'You',
  'clubPage.new': 'Too new to rank',
  'clubPage.unknown': 'Unknown trainer',
  'clubPage.pickHint': 'Select a member to highlight their line on the chart.',
  'clubPage.notLoaded.title': "This club isn't loaded yet",
  'clubPage.notLoaded.detail':
    'Clubs load as searches reach them. Try again later, or go back and search.',

  'chart.heading': 'Pace this month',
  'chart.show': 'Chart shows',
  'chart.daily': 'Daily gain',
  'chart.month': 'Fans this month',
  'chart.labelDaily':
    "Line chart of each member's daily fan gain this month, with the club median and your line. Use the arrow keys to read each day.",
  'chart.labelMonth':
    'Line chart of the fans each member has gained this month, with the club median and your line. Use the arrow keys to read each day.',
  'chart.axisDaily': 'Fans gained per day',
  'chart.axisMonth': 'Fans gained this month',
  'chart.axisDay': 'Day of the month',
  'chart.requirement': 'Requirement',
  'chart.members': 'Members',
  'chart.median': 'Club median',
  'chart.you': 'You',
  'chart.day': ({ n }: { n: number }) => `Day ${n}`,
  'chart.range': ({ low, high }: { low: string; high: string }) => `${low} to ${high}`,
  'chart.empty': 'There are too few days of data for a chart yet.',

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

  'nav.pages': 'Pages',
  'nav.home': 'Home',
  'nav.finder': 'Club finder',
  'wishlist.title': 'Wishlist · Uma Stable Match',
  'wishlist.link': 'Wishlist',
  'wishlist.heading': 'Help decide what gets built next',
  'wishlist.intro':
    "Here are some ideas I'm thinking of building. Vote on the ones you'd actually use.",
  'wishlist.polls': 'Polls',
  'wishlist.poll.jpTrainers': 'JP Trainer and Circles poll',
  'wishlist.aboutLabel': 'About these polls',
  'wishlist.about':
    'These polls are hosted by <a href="https://strawpoll.com" target="_blank" rel="noopener noreferrer">StrawPoll</a>, not by Uma Stable Match. Your vote is stored and handled by StrawPoll under <a href="https://strawpoll.com/privacy" target="_blank" rel="noopener noreferrer">its privacy policy</a>.',
  'club.quickApply': 'Quick Apply',
  'club.quickApplyOff': "This club doesn't have quick applications enabled",
  'club.applies': 'Quick apply',
  'club.appliesTitle': 'You can apply to this club right from Uma Stable Match',

  'applications.link': 'Applications',
  'applications.title': 'Applications · Uma Stable Match',
  'applications.heading': 'Which best describes you?',
  'applications.intro':
    'From club leaders to hopeful applicants, the applications dashboard keeps the next step clear. It brings the tools and information you need into one place, so you can manage a club or follow your application progress without the usual friction.',
  'applications.requirements':
    '<ul><li>Leaders can manage the bot and their club settings when the Discord setup is live.</li><li>Trainers can track their applications and keep an eye on the clubs they want to join.</li></ul>',
  'applications.privacy': ({ href }: { href: string }) =>
    `Your Discord sign-in is only used for club application management. See our <a href="${href}">privacy information</a>.`,
  'applications.session.signIn': 'Sign in with Discord',
  'applications.session.signedIn': 'Your Discord session is active.',
  'applications.role.heading': 'Who are you?',
  'applications.role.leader.title': 'I’m a club leader',
  'applications.role.leader.description':
    'View bot status, check application activity and manage how your club handles Discord applications.',
  'applications.role.trainer.title': 'I’m a trainer',
  'applications.role.trainer.description':
    'Keep track of the clubs you have applied to and see your application history in one place.',

  'applications.error.generic': 'Something went wrong while setting up the club applications flow.',
  'applications.error.wrong_server': 'This Discord server is not set up for this club.',
  'applications.error.not_manager':
    'You need Manage Server or Administrator in the club’s Discord server to add the bot.',
  'applications.error.not_leader': 'Only the club’s leader can add the bot.',
  'applications.error.no_server': 'This club does not have an active Discord server linked yet.',
  'applications.error.denied': 'Discord sign-in was denied. Please try again.',
  'applications.error.invalid_state': 'The Discord sign-in is no longer valid. Please try again.',
  'applications.error.discord_error': 'Discord returned an unexpected error. Please try again.',
  'applications.error.not_open': "This club doesn't take applications through Discord right now.",
  'applications.error.no_trainer':
    'Link your trainer ID in Settings before you apply, so the club can see who you are.',
  'applications.error.bad_club': 'That club link is not valid.',
  'applications.error.already_applied':
    'You already have an application to this club waiting for an answer.',
  'applications.error.too_many_applications':
    "You've sent the most applications you can in a day. Try again tomorrow.",
  'applications.error.join_failed': "Discord wouldn't add you to the club's server.",
  'applications.error.ticket_failed':
    "We couldn't open your ticket on the club's server. Please try again.",
  'applications.error.unavailable':
    'The club applications setup is temporarily unavailable. Please try again later.',
  'applications.error.rate_limited':
    'Too many tries in a short time. Please wait a while and try again.',

  'footer.tipsSummary': 'Tips for switching clubs',
  'footer.tip14':
    '<strong>14-day rule:</strong> You need to have been in a club for at least 14 days in the month to get the monthly reward. Try to move clubs at the start of the month.',
  'footer.tip24':
    '<strong>24-hour cooldown:</strong> If you leave a club, you cannot join a different club for 24 hours. Some people avoid this by asking the club leader to remove them instead.',
  'footer.tipLock':
    '<strong>Month-start lock:</strong> From UTC 10:00 to 14:59 on the first of the month, you cannot leave clubs, join requests or invites are locked.',
  'footer.tipActivity':
    '<strong>Check activity first:</strong> Before applying, look at a club’s member activity on uma.moe so you know how active it is.',
  'footer.tipsNote': 'These rules can change. Check in-game if there is a difference.',
  'footer.links': 'Project links',
  'footer.report': 'Report a problem',
  'footer.privacy': 'Privacy Policy',
  'footer.terms': 'Terms of Service',
  'footer.kofi': 'Buy me a carrot',
  'footer.carats': 'Free carats',
  'footer.support': 'Optional tips help cover hosting. Every feature stays free.',
  'footer.unofficial':
    "Uma Stable Match is an <strong>unofficial, non-commercial fan tool</strong>. It isn't affiliated with, endorsed by or sponsored by uma.moe or Cygames, Inc.",
  'footer.data':
    'Club and trainer data comes from <a href="https://uma.moe" target="_blank" rel="noopener noreferrer">uma.moe</a> and may be delayed or incomplete. Recommendations are estimates to help you decide, not judgements of any player or club.',
  'footer.trademark':
    '<em>Umamusume: Pretty Derby</em> and its names, characters, artwork and trademarks belong to Cygames, Inc. The club tier icons are © Cygames, Inc. and are used non-commercially in line with Cygames\' <a href="https://umamusume.com/fan-createdguide" target="_blank" rel="noopener noreferrer">Fan-Created Content Guidelines</a>.',

  'trainer.title': 'My applications · Uma Stable Match',
  'trainer.eyebrow': 'Club applications',
  'trainer.heading': 'My applications',
  'trainer.lede':
    'The clubs you’ve applied to and where each application stands. Applications expire one month after you send them.',
  'trainer.table.heading': 'Active applications',
  'trainer.table.empty': 'No active applications',
  'trainer.table.emptyHint':
    'Apply to a club from the club finder and your application shows up here for one month.',
  'trainer.table.club': 'Club',
  'trainer.table.trainerId': 'Trainer ID',
  'trainer.table.expires': 'Expires',
  'trainer.table.ticket': 'Open in Discord',
  'trainer.table.ticketFor': ({ name }: { name: string }) =>
    `Open ${name}’s application in Discord`,
  'trainer.table.noTicket': 'Ticket closed',

  'leader.title': 'Club applications · Uma Stable Match',
  'leader.eyebrow': 'Club applications',
  'leader.heading': 'Your club',
  'leader.loading': 'Loading…',
  'leader.signIn': 'Your sign-in has ended. Sign in with Discord to manage this club.',
  'leader.forbidden': 'Only the club’s leader can manage it. Check the trainer linked in Settings.',
  'leader.loadError': 'We couldn’t load this club. Try again in a moment.',
  'leader.linked': 'The bot was added to your server.',
  'leader.state.off': 'Not set up',
  'leader.state.open': 'Taking applications',
  'leader.state.paused': 'Paused',
  'leader.state.problem': 'Needs attention',
  'leader.callout.problem':
    'Players can’t apply right now. Fix the problem shown under Bot status.',
  'leader.callout.paused':
    'Applications are paused. Players don’t see the Apply button for your club.',
  'leader.callout.resume': 'Resume',
  'leader.setup.eyebrow': 'Club applications on Discord',
  'leader.setup.heading': ({ name }: { name: string }) =>
    `Let players apply to ${name} in one click`,
  'leader.setup.yourClub': 'your club',
  'leader.setup.lede':
    'Players who find your club here can apply straight away. You review them in your own Discord server.',
  'leader.flow.applyTitle': 'A player applies',
  'leader.flow.applyText': 'from your club’s page or their search results.',
  'leader.flow.ticketTitle': 'The bot opens a ticket',
  'leader.flow.ticketText': 'a private channel in your server, with their pace and fans.',
  'leader.flow.decideTitle': 'You decide',
  'leader.flow.decideText': 'in Discord, and track every application on this page.',
  'leader.steps.heading': 'Set it up',
  'leader.steps.done': 'Done',
  'leader.steps.todo': 'To do',
  'leader.steps.account': 'Link your Umamusume account',
  'leader.steps.accountHow':
    'Link your trainer ID in Settings. That’s how we know which club you lead: only its leader can add the bot.',
  'leader.steps.accountDone': ({ id }: { id: string }) => `Linked trainer ${id}.`,
  'leader.steps.notLeader':
    'This trainer isn’t the leader of a club. Only a club’s leader can add the bot.',
  'leader.steps.openSettings': 'Open Settings',
  'leader.steps.invite': 'Put an invite to your Discord server in your club description',
  'leader.steps.inviteHow':
    'This is how we know the server belongs to your club. In game, edit your club’s description and paste an invite link, such as discord.gg/yourcode. Any invite to the server works, and it can expire once you’re set up. uma.moe can take a while to show a new description.',
  'leader.steps.inviteFound': 'Found an invite to your server in your club description.',
  'leader.steps.inviteMissing':
    'We didn’t find a working invite to one server in your club description yet.',
  'leader.steps.checking': 'Checking your club description…',
  'leader.steps.checkAgain': 'Check again',
  'leader.steps.permissions': 'Make sure you have Manage Server or Administrator in that server',
  'leader.steps.permissionsHow':
    'Discord only lets people with one of these add a bot. Server owners have both. If you don’t, ask a server admin to do the next step.',
  'leader.steps.addBot': 'Add the bot',
  'leader.steps.addBotHow':
    'Discord opens with your server already picked. Once the bot is added, you come back here to your dashboard.',
  'leader.addBot': 'Add to Discord',
  'leader.docs': 'Read the docs',
  'leader.status.heading': 'Bot status',
  'leader.status.inServer': 'The bot is in your server',
  'leader.status.problem': ({ problem }: { problem: string }) =>
    `Something needs fixing: ${problem}`,
  'leader.status.open': 'Taking applications',
  'leader.status.paused': 'Applications are paused',
  'leader.settings.heading': 'Settings',
  'leader.settings.open': 'Take applications',
  'leader.settings.openHint': 'Off hides the Apply button for your club.',
  'leader.settings.autoJoin': 'Add applicants to the server',
  'leader.settings.autoJoinHint': 'Applicants join your server when they apply.',
  'leader.settings.save': 'Save',
  'leader.settings.saving': 'Saving…',
  'leader.settings.saved': 'Saved.',
  'leader.settings.saveError': 'We couldn’t save the changes.',
  'leader.remove.heading': 'Remove the bot',
  'leader.remove.hint':
    'Stops applications for this club and deletes its link to the server. If no other club uses the server, the bot leaves it.',
  'leader.remove.button': 'Remove the bot from this club',
  'leader.remove.confirm': 'Remove the bot from this club? You can add it again later.',
  'leader.remove.yes': 'Remove',
  'leader.remove.cancel': 'Cancel',
  'leader.remove.error': 'We couldn’t remove the bot right now.',
  'leader.table.heading': 'Applications',
  'leader.table.empty': 'No applications yet',
  'leader.table.emptyHint':
    'When a player applies, their application shows up here and in a ticket channel in your server.',
  'leader.table.trainer': 'Trainer',
  'leader.table.submitted': 'Submitted',
  'leader.table.pace': 'Pace',
  'leader.table.fans': 'Fans this month',
  'leader.table.currentClub': 'Current club',
  'leader.table.noClub': 'None',
  'leader.table.status': 'Status',
  'leader.table.actions': 'Actions',
  'leader.table.view': 'View',
  'leader.table.viewFor': ({ name }: { name: string }) => `View ${name}’s application`,
  'leader.filter.label': 'Filter by status',
  'leader.filter.all': 'All',
  'leader.filter.pending': 'Pending',
  'leader.filter.accepted': 'Accepted',
  'leader.filter.declined': 'Declined',
  'leader.applicationStatus.pending': 'Pending',
  'leader.applicationStatus.accepted': 'Accepted',
  'leader.applicationStatus.declined': 'Declined',
  'leader.details.heading': ({ name }: { name: string }) => `${name}’s application`,
  'leader.details.close': 'Close',
  'leader.details.intro': 'This is what the bot posted to your server.',
  'leader.details.embedTitle': ({ name }: { name: string }) => `New application for ${name}`,
  'leader.details.embedUnnamed': 'New application',
  'leader.details.trainerId': 'Trainer ID',
  'leader.details.discord': 'Discord',
  'leader.details.submitted': ({ date }: { date: string }) => `Submitted ${date}`,
  'leader.details.openTicket': 'Open ticket in Discord',
  'leader.details.compare': 'Compare in club',
  'compare.title': ({ name }: { name: string }) => `Compare applicants · ${name}`,
  'compare.back': 'Back to applications',
  'compare.eyebrow': 'Applicant comparison',
  'compare.intro':
    'See how applicants would fit in your club. Each one is placed among your current members on their own, as if they were the only one joining.',
  'compare.loading': 'Loading the comparison…',
  'compare.empty': 'Add an applicant to see how they’d fit in the club.',
  'compare.unnamed': ({ id }: { id: string }) => `Trainer ${id}`,
  'compare.vsMedian': 'Vs club median',
  'compare.alreadyMember': 'Already in this club; placed as if joining it.',
  'compare.remove': ({ name }: { name: string }) => `Remove ${name}`,
  'compare.req.heading': 'Fan requirement',
  'compare.req.intro':
    'Enter what your club asks of its members to draw it on the chart and see who keeps up.',
  'compare.req.amount': 'Fans (millions)',
  'compare.req.interval': 'Per',
  'compare.req.day': 'Day',
  'compare.req.week': 'Week',
  'compare.req.month': 'Month',
  'compare.req.set': 'Show on chart',
  'compare.req.clear': 'Clear',
  'compare.req.invalid': 'Enter a number of millions, such as 8.',
  'compare.req.meets': ({ gap }: { gap: string }) => `Meets it, ${gap} ahead`,
  'compare.req.short': ({ gap }: { gap: string }) => `Short by ${gap}`,
  'compare.req.summary': ({ pace }: { pace: string }) => `That is ${pace} a day.`,
  'compare.add.heading': 'Add a trainer',
  'compare.add.applicant': 'Applicant',
  'compare.add.choose': 'Choose an applicant…',
  'compare.add.none': 'No other active applications.',
  'compare.add.byId': 'Or a trainer ID',
  'compare.add.button': 'Add',
  'compare.add.invalid': 'Enter a trainer ID: 9 to 12 digits.',
  'compare.add.full': ({ max }: { max: number }) =>
    `You can compare up to ${max} trainers. Remove one to add another.`,
  'leader.details.ticketGone': 'The ticket channel has been closed.',

  'account.label': 'Account',
  'account.signIn': 'Sign in',
  'account.unnamed': 'Discord account',
  'account.signedInAs': ({ name }: { name: string }) => `Signed in as ${name}`,
  'account.settings': 'Settings',
  'account.signOut': 'Sign Out',
  'account.signOutError': 'We couldn’t sign you out. Try again in a moment.',

  'settings.title': 'Settings · Uma Stable Match',
  'settings.heading': 'Settings',
  'settings.lede': 'Manage your Uma Stable Match account.',
  'settings.link.heading': 'Link your umamusume account',
  'settings.link.intro':
    'Prove a trainer ID is yours by adding a short key to its profile comment in the game.',
  'settings.link.label': 'Trainer ID',
  'settings.link.hint': '9 to 12 digits, shown on your trainer profile.',
  'settings.link.invalid': 'Enter a trainer ID of 9 to 12 digits.',
  'settings.link.submit': 'Link Account',
  'settings.link.keyHeading': ({ id }: { id: string }) =>
    `Add this key to the profile comment of trainer ${id}`,
  'settings.link.keySteps':
    'In the game, open your trainer profile, edit your comment and add the key anywhere in it. Then select Check. uma.moe can take a while to show a new comment, and the key works for 24 hours.',
  'settings.link.copy': 'Copy key',
  'settings.link.copied': 'Copied',
  'settings.link.check': 'Check',
  'settings.link.checking': 'Checking…',
  'settings.link.cancel': 'Cancel',
  'settings.link.verifying': 'Verifying',
  'settings.link.verifyingHint':
    'We look for the key on uma.moe now and then every minute for 5 minutes. You can leave this page; it keeps going.',
  'settings.link.missing':
    'We couldn’t find the key in the comment after 5 minutes. Check it shows on uma.moe, then select Check again.',
  'settings.link.linked': ({ name }: { name: string }) =>
    `Linked ${name}. You can take the key out of your comment now.`,
  'settings.link.notFound': 'We couldn’t find that trainer on uma.moe.',
  'settings.link.already': 'That trainer is already linked to you.',
  'settings.link.tooMany': ({ n }: { n: number }) =>
    n === 1
      ? 'You can link one trainer at a time. Unlink it first.'
      : `You can link up to ${n} trainers. Unlink one first.`,
  'settings.link.full': 'You’ve linked a trainer. Unlink it to link a different one.',
  'settings.link.tooManyTries': 'Too many tries. Wait a while and try again.',
  'settings.link.expired': 'That key has expired. Start again for a new one.',
  'settings.link.error': 'Something went wrong. Try again in a moment.',
  'settings.link.linkedHeading': 'Linked trainer',
  'settings.link.none': 'No trainers linked yet.',
  'settings.link.unlink': 'Unlink',
  'settings.link.unlinkFor': ({ id }: { id: string }) => `Unlink trainer ${id}`,
  'settings.link.loadError': 'We couldn’t load your linked trainers.',
  'settings.link.unavailable': 'Linking trainers isn’t available right now.',
  'settings.session.heading': 'Sign out',
  'settings.session.text': 'Ends your session on this device.',
  'settings.signOut': 'Sign Out',
  'settings.delete.heading': 'Delete account',
  'settings.delete.text':
    'Removes your linked trainers and any keys waiting to be checked, then signs you out. Clubs you added the bot to keep it.',
  'settings.delete.button': 'Delete Account',
  'settings.delete.confirm':
    'Delete your account? Your linked trainers are removed and you’re signed out. This can’t be undone.',
  'settings.delete.yes': 'Delete',
  'settings.delete.cancel': 'Cancel',
  'settings.delete.error': 'We couldn’t delete your account right now. Try again in a moment.',
};

/** The messages every language provides. */
export type Catalog = typeof en;
