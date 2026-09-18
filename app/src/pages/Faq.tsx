import { useState } from "react";

// Answers grounded in README.md and company/*.md. No marketing fluff.

const QA: { q: string; a: string }[] = [
  {
    q: "Do I need an account to use Ferry?",
    a: "No. Backup, USB creation, OS downloads and restore are free forever with no login wall. An account is only needed for the planned Cloud Backup feature.",
  },
  {
    q: "Will I lose my files when my PC is wiped?",
    a: "Not if you follow the order Ferry enforces: back up → checksum-verify → erase. The erase step stays locked until every backed-up file is verified byte-for-byte against the original.",
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
    a: "In this build you need a bigger drive. The planned Cloud Backup feature will hold the overflow (or a second safety copy) in the cloud for a one-time fee — see Pricing.",
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
    a: "No. In this build everything stays between your PC and your USB drive: OS images come straight from vendor servers, and your files never leave the encrypted backup on the stick.",
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
