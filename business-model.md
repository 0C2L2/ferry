# Business Model

## The core app is free, completely

Ventoy and Rufus already do the "download an OS and burn a bootable USB" job for free, forever, open source — that function cannot be charged for. Donations alone don't make a sustainable business either (most open-source maintainers go unpaid). So Ferry's client software — OS download, bootable USB creation, personal-file backup — is free, no login wall, by design. That is the distribution engine, not a stripped-down teaser.

## Where the money comes from: cloud storage overflow

When a user's personal data doesn't fit on the USB alongside the OS image, Ferry offers to rent cloud storage for the overflow, backed by Backblaze B2 (see [`architecture.md`](architecture.md) for the technical numbers).

**Positioning matters**: this is a **migration bridge**, not an ongoing backup subscription. Backblaze itself already sells unlimited consumer cloud backup for \$9/month — competing there means fighting Backblaze, Dropbox, Google One, and iCloud on their own turf with none of their scale. Instead, Ferry's cloud add-on solves a narrower, time-limited problem: "my data doesn't fit right now, for this one migration." Upload before the wipe, download after restore, then let it expire.

**Rough pricing shape**: B2's actual cost is well under \$1 to hold 50 GB for a month, with restore-downloads essentially free under B2's own free-egress-up-to-3× policy. That leaves room for a simple one-time fee scaled to overflow size:

| Overflow size | Suggested price |
|---|---|
| Up to 50 GB | ~\$5 |
| Up to 200 GB | ~\$15 |
| Up to 1 TB | ~\$40 |

A flat one-time charge is a much easier "yes" than a new recurring subscription for someone in the middle of wiping their PC.

> [!NOTE]
> These prices are illustrative. Actual pricing should be validated against B2's real-time rate card and user willingness-to-pay research before launch.

## A B2B angle considered, not yet the current plan

Ninite's split — free installer for individuals, paid "Ninite Pro" subscription sold per-machine to IT departments — is a strong template if Ferry ever wants a second revenue line aimed at repair shops and IT departments (who reinstall OSes constantly, unlike a consumer who does it once every few years). Worth revisiting post-MVP; the cloud-storage model is the current primary plan.

## Two rules, decided on purpose

- **Never monetize the data itself.** The product's entire pitch is "trust us with your files during the scariest ten minutes your computer will have" — including saved passwords and product keys. No data monetization, ever.
- **Never let payment influence the app-picker's ranking.** The three-tier trust system exists specifically to protect people from fake-download-site scams. A sponsored result that could outrank a genuinely official source defeats the entire point of building the picker.
