import { useState } from "react";

// Answers grounded in README.md and company/*.md. No marketing fluff.

const QA: { q: string; a: string }[] = [
  {
    q: "Do I need an account to use Ferry?",
    a: "No. Backup, USB creation, OS downloads and restore are free forever with no login wall. You only register and sign in if you buy the Cloud Backup add-on.",
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
    a: "Use a larger drive — Ferry tells you the required size before anything is erased. Or buy the Cloud Backup add-on (one-time per migration, priced by backup size — see Pricing): it holds the overflow, or a second safety copy, in the cloud.",
  },
  {
    q: "Are my browser passwords transferred?",
    a: "Bookmarks transfer. Saved passwords from Chrome, Edge or Brave usually do not survive a reinstall because Windows encrypts them to the old system (DPAPI). Use your browser's sync feature for passwords; Ferry keeps the files for reference. Firefox needs its Primary Password if one was set.",
  },
  {
    q: "Will my programs come back automatically?",
    a: "No — installed programs are never copied over, because that breaks across systems. Ferry gives you a checklist of what was installed with one-click reinstall buttons from verified sources (winget catalog first, vendor site second, or an honest “no source found”).",
  },
  {
    q: "Is any of my data uploaded to the internet?",
    a: "Only if you buy the optional Cloud Backup add-on: then your already-encrypted backup file is uploaded to Ferry's storage — Ferry cannot read it, and it is deleted after you restore. Everything else stays between your PC and your USB drive.",
  },
  {
    q: "Why does Ferry need administrator access?",
    a: "Partitioning and formatting a USB drive, exporting Wi-Fi profiles, and scanning installed drivers are privileged Windows operations. Ferry only uses elevation for these steps.",
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
