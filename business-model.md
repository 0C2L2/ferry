# Business Model

## The core app is free, completely

Ventoy and Rufus already do the "download an OS and burn a bootable USB" job for free, forever, open source — that function can't be charged for. Donations alone don't make a sustainable business either (most open-source maintainers go unpaid). So Ferry's client software — OS download, bootable USB creation, personal-file backup — is free, no login wall, by design. That's the distribution engine, not a stripped-down teaser.

## Where the money comes from: cloud storage overflow

When a user's personal data doesn't fit on the USB next to the OS image, Ferry offers to rent cloud storage for the overflow, backed by Backblaze B2 (see `architecture.md` for the technical numbers).

**Positioning matters here**: this is a **migration bridge**, not an ongoing backup subscription. Backblaze itself already sells unlimited consumer cloud backup for $9/month — competing there means fighting Backblaze, Dropbox, Google One, and iCloud on their own turf with none of their scale. Instead, Ferry's cloud add-on solves a narrower problem: "my data doesn't fit right now, for this one migration." Upload before the wipe, download after restore, then let it expire.

**Rough pricing shape**: B2's actual cost is well under $1 to hold 50GB for a month, with restore-downloads essentially free under B2's own free-egress-up-to-3x policy. That leaves room for a simple one-time fee scaled to overflow size (e.g., up to 50GB extra: ~$5, up to 200GB: ~$15) rather than a new recurring charge — a much easier yes for someone mid-panic about wiping their PC.

## A B2B angle considered, not yet the current plan

Ninite's split — free installer for individuals, paid "Ninite Pro" subscription sold per-machine to IT departments — is a strong template if Ferry ever wants a second revenue line aimed at repair shops and IT departments (who reinstall OSes constantly, unlike a consumer who does it once every few years). Worth revisiting later; the cloud-storage model is the current primary plan.

## Two rules, decided on purpose

- **Never monetize the data itself.** The product's entire pitch is "trust us with your files during the scariest ten minutes your computer will have" — including, now, saved passwords and product keys. No data monetization, ever.
- **Never let payment influence the app-picker's ranking.** The three-tier trust system exists specifically to protect people from fake-download-site scams. A sponsored result that could outrank a genuinely official source defeats the entire point of building it.
