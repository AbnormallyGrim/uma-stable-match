# Privacy

What Uma Stable Match collects, why, who sees it, and how to have it removed.

Last updated: 8 November 2026

## 1. Who runs this site

Uma Stable Match is an unofficial, non-commercial fan tool run by Abnormally Grim. It isn't affiliated with uma.moe or Cygames, Inc. There is no company and no Data Protection Officer or Information Officer; the operator is the person responsible for your data.

Privacy questions and requests: privacy@umastable.com. Please don't send personal data through public GitHub issues.

## 2. What we collect

**What you give us**

- The trainer ID you search for or link.
- Your Discord ID, when you sign in with Discord. We look up your Discord display name for the account menu but don't store it.
- Your club applications: the club, your trainer ID and name, your current club, your pace and fans, and when you applied.
- For club leaders: the club's Discord server, ticket category and settings.

**What we collect automatically**

- Your IP address, kept in memory only while it is rate limited. It is never written to logs.
- Each search is written to our server logs with its trainer ID and search terms.
- Error and performance reports from our servers and from your browser, which can include the address of the page you were on. A search address can contain a trainer ID.
- Page view counts from Cloudflare Web Analytics, without cookies.
- When the human check is on, Cloudflare Turnstile checks your browser to tell you from a bot.

**From public sources.** Trainer and club data comes from [uma.moe](https://uma.moe): names, fans, pace, rank and profile comments. We read your profile comment to check that you own a trainer ID.

**Payments.** We don't take any. Optional tips go through Ko-fi, on its own site.

## 3. Why we use it

| <tr><th scope="col">Purpose | Data | Legal basis |
|---|---|---|
| Recommending clubs | Trainer ID, public uma.moe data | You asked for it (the service) |
| Signing in, linking a trainer, applying, managing a club | Discord ID, trainer ID, application details | You asked for it (the service) |
| Stopping abuse | IP address, human-check token | Legitimate interest |
| Fixing errors and keeping the site fast | Logs, error and performance reports | Legitimate interest |
| Counting visits | Cloudflare Web Analytics page views | Legitimate interest |

We don't use your data for marketing, advertising or profiling.

## 4. Cookies and browser storage

Every cookie here is needed for the site to work. There are no analytics or advertising cookies, so there is no cookie banner.

| <tr><th scope="col">Cookie | Purpose | Lasts |
|---|---|---|
| `usm_human` | Shows you passed the human check. Holds nothing about you | 10 minutes |
| Session | Keeps you signed in. Signed, and holds your Discord ID | 12 hours |
| Signed in | Tells pages you're signed in, so they needn't ask the server each time. Holds only when your session ends | 12 hours |
| Sign-in check | Protects the Discord sign-in | 10 minutes |
| Language | Remembers your language, when the language switch is offered | 1 year |

Your browser's local storage, which never leaves your device, holds your theme, your language, your trainer ID if you choose to remember it, your Discord ID and display name while you're signed in, whether you've seen the tour and release notes, when your human check expires, and where the club page should return to.

You can clear cookies and local storage in your browser's settings. Clearing the session cookie signs you out.

## 5. Who we share it with

- **uma.moe** receives the trainer IDs you look up, to fetch public game data.
- **Discord** handles sign-in and, when you apply, adds you to the club's server. Our bot posts your application in a private ticket channel that you, the club's leader and the bot can see. It shows your trainer name, a uma.moe profile link and your stats. Once it is in Discord, the ticket is under Discord's policy and the club server's control, and we can't delete it.
- **Club leaders** see your applications to their club, and can compare up to five applicants with their current members using public data.
- **Service providers:** Fly.io (hosting and our database), Sentry (errors, logs and performance reports), Cloudflare (Web Analytics and the human check), Google Fonts (the website's fonts, which sees your IP address) and StrawPoll (the wishlist polls, under StrawPoll's own policy).

We don't sell or rent personal data, and we don't share it for advertising across sites.

Our servers are in the United States, as is Sentry, so your data leaves your country.

## 6. Security and how long we keep it

The site uses HTTPS only. The session cookie is signed and hidden from page scripts. We store Discord IDs only as a keyed hash or encrypted, our database has a password and no public address, and Discord's access token is used once and thrown away.

| <tr><th scope="col">Data | Kept for |
|---|---|
| IP address | Minutes, in memory only |
| Looked-up trainer pace | 10 minutes, stored under a hash of the trainer ID |
| Pending verification key | 24 hours |
| Linked trainer ID and name | Until you unlink it or delete your account |
| Application | One calendar month after you send it |
| Club link and settings | Until the leader unlinks the club or the bot is removed from the server |
| Search logs, error and performance reports | Kept by Sentry, under its [privacy policy](https://sentry.io/privacy/) |

## 7. Your rights

You can do these yourself:

- [Delete your account](https://app.umastable.com/settings), which removes your linked trainer and ends all your sessions.
- Unlink a trainer or a club on the same pages.
- See your applications under [My applications](https://app.umastable.com/applications/trainer).
- Remove the app from your Discord account, which signs you out everywhere.

Deleting your account doesn't remove pending applications, which expire after a month, clubs you opted in, which stay until you unlink them, or tickets already in Discord.

To ask for a copy of your data, to correct it or to have anything else deleted, email privacy@umastable.com. We reply within 30 days. You can withdraw consent or object to processing the same way. If you're unhappy with our answer, you can complain to your local data protection authority.

## 8. Changes to this policy

The date at the top shows when it last changed. We'll announce material changes in the app's release notes. We don't hold your email address, so we can't write to you directly.
