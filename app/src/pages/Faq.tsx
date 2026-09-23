import { useState } from "react";

// Answers grounded in README.md and company/*.md. No marketing fluff.

const QA: { q: string; a: string }[] = [
  {
    q: "Do I need an account to use Ferry?",
    a: "No. Backup, USB creation, OS downloads and restore are free with no sign-up. Even the optional cloud copy needs no account: you get a restore code instead.",
  },
  {
    q: "Will I lose my files when my PC is wiped?",
    a: "The USB drive is prepared first, while it holds nothing of yours yet — your PC is never touched by that step. Afterwards every file is copied, checksum-verified, and encrypted before anything else happens. Nothing is erased that hasn't been verified.",
  },
  {
    q: "What happens if I forget my backup password?",
    a: "The backup cannot be opened — there is no recovery, by design. Write the password down before starting; the app makes you confirm you did.",
  },
  {
    q: "Will Windows activate again after reinstalling?",
    a: "Yes, as long as you reinstall the same edition (e.g. Home stays Home) on the same hardware. The digital license is tied to the hardware, not the wiped drive. Ferry shows the detected edition so you download the matching image.",
  },
  {
    q: "My USB drive is too small. What now?",
    a: "Use a larger drive — Ferry tells you the required size before anything is erased. Or use Ferry Cloud Backup — free for now, up to 50 GB, kept 30 days: it holds a second encrypted copy in the cloud.",
  },
  {
    q: "Are my browser passwords transferred?",
    a: "Bookmarks transfer. Saved passwords from Chrome, Edge or Brave usually do not survive a reinstall because Windows encrypts them to the old system (DPAPI). Use your browser's sync feature for passwords; Ferry keeps the files for reference. Firefox needs its Primary Password if one was set.",
  },
  {
    q: "Will my programs come back automatically?",
    a: "Programs themselves are never copied — that breaks across systems. Ferry lists the apps you installed yourself; on Ubuntu it installs the Linux versions it can with one password prompt, and marks the rest honestly (a labelled alternative, or no Linux version).",
  },
  {
    q: "Is any of my data uploaded to the internet?",
    a: "Only if you choose the optional cloud copy: then your already-encrypted backup file is uploaded to Ferry's storage — Ferry cannot read it, and it is deleted after 30 days or when you restore it. Everything else stays between your PC and your USB drive.",
  },
  {
    q: "Why does Ferry need administrator access?",
    a: "Partitioning and formatting a USB drive, exporting Wi-Fi profiles, and reading the list of installed apps are privileged Windows operations. Ferry only uses elevation for these steps.",
  },
  {
    q: "Do my Wi-Fi networks come back?",
    a: "Yes. Saved networks (including keys) are exported before the wipe and reimported automatically during restore, inside the same encrypted backup.",
  },
];

export function Faq() {
  const [open, setOpen] = useState<number | null>(0);
  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-2xl font-bold text-center mb-6">Frequently asked questions</h2>
      <div className="space-y-2">
        {QA.map((item, i) => (
          <div key={item.q} className="bg-white border border-gray-200 rounded-xl overflow-hidden">
            <button
              onClick={() => setOpen(open === i ? null : i)}
              className="w-full text-left px-5 py-3.5 font-medium flex justify-between items-center gap-3"
            >
              {item.q}
              <span className="text-gray-400 shrink-0">{open === i ? "−" : "+"}</span>
            </button>
            {open === i && <p className="px-5 pb-4 text-sm text-gray-600">{item.a}</p>}
          </div>
        ))}
      </div>
    </div>
  );
}
