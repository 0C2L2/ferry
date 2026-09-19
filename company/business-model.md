# Business Model

## Everything is free, including Cloud Backup

Ferry has no paid tier. The client software — OS download, bootable USB creation, personal-file
backup, restore, app/driver inventory — is free, no login wall, by design. **Cloud Backup is
also free**: it uploads the user's already-encrypted `Backup.enc` to **Ferry's own** Backblaze
B2 storage — the user never creates an account, bucket, or key. Ferry absorbs the storage cost
itself (see `architecture.md` for why B2's free-egress tier keeps this affordable) rather than
billing for it or requiring the user to bring their own cloud account. The tradeoff made
explicitly for this: Ferry deletes each backup from its cloud storage once the user has
successfully restored it, so it isn't accumulating everyone's data indefinitely.

An account (`app/src/auth.tsx`) is purely an optional local profile — it is never required by
any feature, including Cloud Backup, and there is no subscription, no billing, and no tier to
choose. This replaces an earlier plan (tiered pricing by backup size, plus a paid Corporate/SMB
subscription) that is no longer the direction for this build.

---

## Two rules, decided on purpose

- **Never monetize the data itself.** The product's entire pitch is "trust us with your files
  during the scariest ten minutes your computer will have" — including saved passwords and
  product keys. No data monetization, ever.
- **Never let payment influence the app-picker's ranking.** The three-tier trust system exists
  specifically to protect people from fake-download-site scams. There is no payment path in
  Ferry that could influence which download source the app picker recommends, and there never
  should be.

---

## If monetization is revisited later

Nothing above forecloses a future paid offering (e.g. a per-seat dashboard for repair shops/IT
departments that reinstall machines regularly, or eventually charging for cloud storage at a
scale where absorbing the cost stops making sense). That would be a deliberate, separate
decision — not a default this document assumes — and should be re-evaluated against the two
rules above before it happens.
