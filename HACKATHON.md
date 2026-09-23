# Ferry at Habsida Hackathon 2026

**When:** Thu 24 Sep 11:00 → Fri 25 Sep 15:00 (KST) · Habsida Space, Incheon
**Format:** startup competition. Idea pitching → teams of 3–5 → Lean Startup,
build overnight, pitch on day 2 to five founders/CEOs. 1st prize ₩1.5M. An MOU
signing ceremony follows the pitches.

## The story we're selling

> In 18 days, Windows 10 gets its last security update ever. More than a
> quarter of Windows PCs still run it, and many can't upgrade to Windows 11.
> Ferry turns them into safe Ubuntu machines in one evening without losing a
> file. Free for people, paid for organisations.

- **Why now:** Microsoft's paid Windows 10 security updates (ESU) end on
  **13 October 2026**, the last date for any consumer PC.
- **How many:** Windows 10 is ~28% of Windows desktops (StatCounter, June 2026).
- **Impact:** Canalys estimated ~240 million PCs could be scrapped because of
  Windows 10's end. *(Check the source before putting it on a slide.)*

**Three rules for the weekend**

1. **Pitch a business, not a Linux tool.** The customer is anyone stuck with
   Windows 10 PCs they can't afford to replace: schools and academies, small
   businesses, NGOs, PC refurbishers, the public sector. Individuals get it free.
2. **Be open about what's new.** Ferry existed before. The weekend build is
   *Ferry Check* and *Ferry for Teams* (below). A git tag proves the difference.
3. **Evidence beats claims.** Interviews, a room poll and pilot sign-ups
   collected during the event go on a slide.

## Tonight (Wed) — before 11:00 Thursday

| # | Who | Task |
|---|-----|------|
| 1 | You | Ask the organisers: (a) may we extend an existing open-source product? (b) pitch length and judging criteria? (c) can we demo a laptop on the projector? |
| 2 | Me  | Commit everything and tag `hackathon-start`, so the weekend's work is provable. |
| 3 | Me  | Smoke-test the live site, download and cloud backup. Nothing risky changes. |
| 4 | Me  | Pitch deck v1 (story below, every number with its source). |
| 5 | Me  | 60-second demo video. You record ~30 s of the real app on your PC; I add the rest. |
| 6 | You | Practise the 60-second idea pitch (bottom of this file). Then sleep; tomorrow night is overnight. |

## Day 1 — Thursday 24 Sep

**Morning: idea pitching and team.** Give the 60-second pitch and recruit:
- a business/marketing person (interviews, market size, pitch);
- a designer (deck, report screen);
- ideally a Korean speaker, to reach local schools, companies and refurbishers;
- optionally one more developer.

**After the Lean Startup lecture (30 min):** fill in a Lean Canvas and write
down the hypotheses to test:
- **H1:** Windows 10 owners fear losing files and apps more than they dislike Linux.
- **H2:** Organisations with old PCs would pay per PC rather than buy new ones.
- **H3:** People will try Ubuntu if someone guarantees everything comes back.

**Afternoon: validate (business teammate).**
- 10+ interviews, at the venue and online. Log each one in a shared sheet.
- A room poll: "Who still has a Windows 10 PC at home or at work?"
- Count sign-ups on the pilot page (B2).

**Build (developer + me). All of it is new this weekend:**
- **B1 · Ferry Check** (Thu afternoon). The app's first screen answers in about
  10 seconds:
  - *Can this PC run Windows 11?* (supported CPU, TPM 2.0, Secure Boot, 4 GB RAM, 64 GB disk)
  - *Is it ready for Ubuntu?* (disk space, graphics card, Wi-Fi adapter, BitLocker)
  - *Your apps:* "23 of 27 come across, 4 have alternatives", using the
    234-app catalog. You tick the ones you want (Phase 7 step 4).
  - One-page report to save or share. This is the **live demo** and the free funnel.
- **B2 · Ferry for Teams dashboard** (Thu night). An organisation gets a team
  code; each PC runs Ferry Check with it; `ferryapp.download/teams` shows the
  fleet: how many PCs can't get Windows 11, how many are ready for Ubuntu, the
  top blocking apps, and the savings. The page also carries a **"Join the
  pilot"** form. This is the paid product, per PC.
  - **Demo moment:** Check on your laptop with the team code → the dashboard
    on the projector goes from 40 to 41 PCs. The 40 are a labelled demo org.
- **Extras, only if B1 and B2 work** (Fri early morning):
  - savings / e-waste calculator on `/teams` (~1 h, labelled as estimates);
  - Korean landing page (~2 h);
  - a "Try Ubuntu first, no install" message (~15 min);
  - AI suggestions for unrecognised apps, labelled "unverified", never
    auto-installed (~3 h, Cloudflare's free AI allowance).
- **Not this weekend:** payments, dual-boot, other operating systems.

**Evening:** after the presentation lecture, show mentors deck v1 and the demo.
Ask them "what would stop you investing?" and fix that.

**Night:**
- Aim to freeze B1/B2 by 03:00.
- Build and test the new installer, keeping the old one as a fallback.
- Record the final video with Ferry Check in it; deck v2.

## Day 2 — Friday 25 Sep

- **Morning:** code freeze, no more changes. Put the latest numbers
  (interviews, sign-ups, poll) into the deck. Rehearse three times with a timer.
- **Q&A drill:** practise the answers below until they take under 20 seconds each.
- **Pitch.** Bring the deck as a PDF, the video offline, and a phone hotspot.
- **MOU ceremony (optional):** a one-page pilot agreement, for example a free
  migration of the host's own old Windows 10 PCs, if they have any.

## The pitch (5 min; a 3-min cut drops sections 7 and 8)

1. **Hook (20 s):** "18 days." Windows 10's last security update.
2. **Problem (40 s):**
   - Today's choice: buy new PCs, stay unprotected, or switch to Linux and risk losing everything.
   - Plus the e-waste.
3. **Solution + demo (90 s):** Ferry Check live on the laptop, then the migration video.
4. **Why it works (30 s):**
   - Encrypted before anything leaves the PC.
   - Every file checked before the drive is erased.
   - 234 apps in a verified catalog; every install is checked to exist.
   - No AI guessing when installing as administrator.
   - Open source.
5. **Business model (40 s):**
   - Free for individuals (the funnel).
   - Cloud Backup: paid per migration.
   - **Ferry for Teams:** per PC, for organisations.
   - Refurbisher partnerships.
6. **Traction and validation (30 s):**
   - A live product and website.
   - Downloads, interviews, pilot sign-ups and the poll result, all from this weekend.
7. **Market and go-to-market (20 s):** Korea first (schools, academies, small
   businesses), then Europe, where governments are moving off Microsoft.
   *(Verify examples first.)*
8. **Team and ask (20 s):** pilot partners, an MOU, mentors.

## Q&A — have these answers ready

- **How do you make money if it's open source and free?** Open source earns the
  trust needed to touch people's files. Organisations pay for Teams (many PCs,
  a dashboard, reports), and individuals pay for Cloud Backup.
- **Why would anyone switch to Linux?** Most won't choose to; they're forced by
  the deadline and by price. Ferry removes the fear of losing their stuff.
- **What if my app has no Linux version?** Ferry says so honestly, or offers a
  labelled alternative. It never pretends.
- **Is it safe? What if the backup fails?** Everything is encrypted, and every
  file is checked before the drive is erased.
- **Competitors?** Windows-to-Windows tools (such as PCmover) exist. *Check
  tonight* whether anything else does Windows → Ubuntu with apps, Wi-Fi and
  restore.
- **What did you build this weekend?** Ferry Check and Teams. The git tag
  `hackathon-start` shows the exact difference.
- **Why no AI?** Its answer becomes an install command run as administrator.
  AI helps us curate the app list offline; a person reviews it and a script
  verifies it.
- **What does it cost to run?** It runs on free tiers today (Cloudflare,
  Backblaze B2), so costs only grow once people pay.

## Risks

| Risk | Plan |
|------|------|
| Bringing an existing product isn't allowed | Pitch Check + Teams as the weekend build, with Ferry core as "our open-source foundation". The tag proves it. |
| The live demo fails | Pre-recorded video, the deck as an offline PDF, a phone hotspot. |
| The new installer breaks | Build and test it Thursday night; the current installer stays on the site. |
| Email sign-in still isn't verified | Demo cloud sign-in with the restore code instead. |

## Who does what

- **You:** the pitch, recruiting, product decisions, presenting the demo.
- **Claude (through you):** code for B1/B2, the deck, the video, research with
  sources, Q&A drafts.
- **Teammates:** interviews and market research, design, the Korean market.

## The 60-second idea pitch (Thursday morning)

> Hi, I'm ___. In 19 days, on October 13th, Windows 10 gets its last security
> update ever. More than a quarter of Windows PCs still run it, and many of
> them can't upgrade to Windows 11. Their owners have three bad options: buy a
> new PC, stay unprotected, or switch to Linux and risk losing everything.
>
> I built Ferry. Plug in one USB drive and it backs up your files, installs
> Ubuntu, and brings back your files, apps, Wi-Fi and bookmarks. It's
> encrypted, it's free, and it already works: ferryapp.download.
>
> This weekend I want to turn it into a business. First, a 10-second check
> that tells any PC owner what happens to their apps. Second, a paid version
> for schools and companies with dozens of old PCs.
>
> I'm looking for a business/marketing person, a designer, and a Korean
> speaker to talk to local schools and companies. Come find me.
