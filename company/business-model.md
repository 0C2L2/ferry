# Business Model

## The core app is free, completely

Ventoy and Rufus already do the "download an OS and burn a bootable USB" job for free, forever, open source — that function cannot be charged for. Donations alone don't make a sustainable business either (most open-source maintainers go unpaid). So Ferry's client software — OS download, bootable USB creation, personal-file backup — is free, no login wall, by design. That is the distribution engine, not a stripped-down teaser.

---

## Where the money comes from: cloud backup

Ferry's one paid feature is **Cloud Backup** — storing the user's files in the cloud during a migration. It is the same underlying feature regardless of why a user activates it. Users register and sign in to access Cloud Backup; the core app never requires an account.

### Two entry points into the same feature

Cloud Backup gets surfaced to the user in one of two ways:

1. **Overflow** — the USB is too small to hold both the OS image and all the user's files. Ferry detects this automatically and offers Cloud Backup to handle the portion that doesn't fit. *Triggered by necessity.*
2. **Extra Careful** — the USB is large enough on its own, but the user wants a second independent encrypted copy in the cloud as a safety net before the wipe. Ferry proactively offers this as a one-tap upsell. *Triggered by choice.*

Both entry points use the same billing, the same infrastructure, and the same encryption. From the user's perspective it is one feature; from a product perspective the entry points are two different sales moments.

> [!NOTE]
> Keeping "overflow" and "extra careful" as a single feature (not two separate SKUs) prevents user confusion. A user shouldn't need to understand the distinction — they just see "Cloud Backup" and a price based on their backup size.

### The two plans

**Individual plan** — for personal users. One-time charge per migration, priced by total backup size:

| Total backup size | Suggested price |
|---|---|
| Up to 50 GB | ~\$5 |
| Up to 200 GB | ~\$15 |
| Up to 1 TB | ~\$40 |

Priced by **total backup size** (not just the overflow portion) for simplicity — the user doesn't need to understand what "overflow" means to buy this.

A flat one-time charge — not a subscription — is the easiest possible "yes" for someone in the middle of wiping their PC.

**Corporate / SMB plan** — for small businesses, IT departments, and repair shops that run migrations regularly. Priced per seat or per migration volume on a monthly or annual subscription. Exact pricing to be validated with early B2B customers; the Ninite Pro model (free for individuals, paid per-machine for businesses) is the benchmark template.

> [!NOTE]
> For corporate customers, Cloud Backup can be framed as a "redundant copy for compliance and audit purposes" rather than "extra careful" — same feature, language matched to audience.

### Cloud infrastructure — split by plan tier

The backend choice differs between plans because the cost structure and customer requirements differ:

**Individual plan → Backblaze B2**
- Storage: ~\$6–7/TB/month. Holding 200 GB for one month costs ~\$1.40.
- Egress: **free** up to 3× whatever is stored that month — a full restore after a migration comfortably fits inside this allowance.
- This is why B2 works for one-time migrations: the egress-free policy means Ferry's margin isn't eaten by the restore download.

**Corporate / SMB plan → Google Cloud Storage, AWS S3, or Azure Blob**
- These providers offer enterprise-grade SLA, global redundancy, and audit trails that business customers require.
- Egress is not free (~\$0.08–0.12/GB depending on provider and region), so Corporate plan pricing must account for it — this is one reason Corporate pricing is subscription-based rather than one-time.
- Ferry charges at a margin above the provider's raw cost.

> [!IMPORTANT]
> Don't use GCS/S3/Azure for the Individual plan. Their egress fees would cost more than the user's one-time payment on any restore over ~50 GB. B2's free-egress model is load-bearing for the Individual unit economics.

**Both plans:** Files are encrypted client-side (AES-256) before upload. The encryption key is derived from the user's password and never transmitted to Ferry's servers — Ferry cannot read the contents of a user's cloud backup.

> [!NOTE]
> Infrastructure pricing should be benchmarked against actual B2, GCS, S3, and Azure rate cards before finalizing user-facing prices. The figures above are illustrative.

---

## A B2B angle — not yet the current plan

The Corporate / SMB plan above is the beginning of a B2B line, but the full potential is larger. Repair shops and IT departments reinstall OSes constantly, unlike a consumer who does it once every few years. A seat-based or volume-based subscription with a management dashboard (track migrations, see backup status per machine) could be a strong second product. Worth building out after the Individual plan is validated.

---

## Two rules, decided on purpose

- **Never monetize the data itself.** The product's entire pitch is "trust us with your files during the scariest ten minutes your computer will have" — including saved passwords and product keys. No data monetization, ever.
- **Never let payment influence the app-picker's ranking.** The three-tier trust system exists specifically to protect people from fake-download-site scams. A sponsored result that could outrank a genuinely official source defeats the entire point of building the picker.
