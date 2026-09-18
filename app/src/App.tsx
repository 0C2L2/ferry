import { useState } from "react";
import { AuthProvider, useAuth } from "./auth";
import { Welcome } from "./pages/Welcome";
import { DriveSelect } from "./pages/DriveSelect";
import { OSSelect } from "./pages/OSSelect";
import { ExcludeReview } from "./pages/ExcludeReview";
import { PasswordSetup } from "./pages/PasswordSetup";
import { BackupProgress } from "./pages/BackupProgress";
import { DownloadProgress } from "./pages/DownloadProgress";
import { WriteProgress } from "./pages/WriteProgress";
import { Done } from "./pages/Done";
import { RestoreDetect } from "./pages/RestoreDetect";
import { RestoreProgress } from "./pages/RestoreProgress";
import { AppPicker } from "./pages/AppPicker";
import { Pricing } from "./pages/Pricing";
import { Faq } from "./pages/Faq";
import { Account } from "./pages/Account";
import type {
  BackupLocation,
  DriveInfo,
  Manifest,
  OsSource,
  RestoreSummary,
  ScanResult,
} from "./types";

type Step =
  | "welcome"
  | "drives"
  | "os"
  | "excludes"
  | "password"
  | "backup"
  | "download"
  | "write"
  | "done"
  | "restore-detect"
  | "restore"
  | "apps";

const BACKUP_BACK: Partial<Record<Step, Step>> = {
  drives: "welcome",
  os: "drives",
  excludes: "os",
  password: "excludes",
};

export default function App() {
  return (
    <AuthProvider>
      <Shell />
    </AuthProvider>
  );
}

type View = "wizard" | "pricing" | "faq" | "account";

function Shell() {
  const { account } = useAuth();
  const [view, setView] = useState<View>("wizard");
  const [step, setStep] = useState<Step>("welcome");
  const [drive, setDrive] = useState<DriveInfo | null>(null);
  const [os, setOs] = useState<OsSource | null>(null);
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [excludes, setExcludes] = useState<string[]>([]);
  const [password, setPassword] = useState("");
  const [manifest, setManifest] = useState<Manifest | null>(null);
  const [bootloaderWarning, setBootloaderWarning] = useState<string | null>(null);
  const [backupLoc, setBackupLoc] = useState<BackupLocation | null>(null);
  const [restorePassword, setRestorePassword] = useState("");
  const [summary, setSummary] = useState<RestoreSummary | null>(null);
  const [stagingDir, setStagingDir] = useState<string | null>(null);

  function restart() {
    setStep("welcome");
    setDrive(null);
    setOs(null);
    setScan(null);
    setExcludes([]);
    setPassword("");
    setManifest(null);
    setBootloaderWarning(null);
    setBackupLoc(null);
    setRestorePassword("");
    setSummary(null);
    setStagingDir(null);
  }

  const back = view === "wizard" ? BACKUP_BACK[step] : undefined;
  const nav = (v: View, label: string) => (
    <button
      key={v}
      onClick={() => setView(v)}
      className={`text-sm px-1 ${view === v ? "text-gray-900 font-semibold" : "text-gray-500 hover:text-gray-900"}`}
    >
      {label}
    </button>
  );

  return (
    <div className="min-h-screen bg-gray-50 text-gray-900 p-8">
      <div className="max-w-4xl mx-auto">
        <header className="mb-8 flex items-center justify-between flex-wrap gap-3">
          <div>
            <h1 className="text-3xl font-bold">Ferry</h1>
            <p className="text-gray-600 text-sm">Secure Backup & OS Reinstallation Assistant</p>
          </div>
          <nav className="flex gap-4 items-center">
            {nav("wizard", "Migrate")}
            {nav("pricing", "Pricing")}
            {nav("faq", "FAQ")}
            {nav("account", account ? account.name.split(" ")[0] : "Sign in")}
          </nav>
          <div className="flex gap-3 items-center">
            {back && (
              <button
                onClick={() => setStep(back)}
                className="text-sm text-gray-600 hover:text-gray-900"
              >
                ← Back
              </button>
            )}
            {view === "wizard" && step !== "welcome" && (
              <button
                onClick={restart}
                className="text-sm text-gray-600 hover:text-gray-900"
              >
                Start over
              </button>
            )}
          </div>
        </header>

        {view === "pricing" && <Pricing onAccount={() => setView("account")} />}
        {view === "faq" && <Faq />}
        {view === "account" && <Account />}
        {view === "wizard" && step === "welcome" && (
          <Welcome onBackup={() => setStep("drives")} onRestore={() => setStep("restore-detect")} />
        )}
        {view === "wizard" && step === "drives" && (
          <DriveSelect selected={drive} onSelect={setDrive} onNext={() => setStep("os")} />
        )}
        {view === "wizard" && step === "os" && (
          <OSSelect selected={os} onSelect={setOs} onNext={() => setStep("excludes")} />
        )}
        {view === "wizard" && step === "excludes" && (
          <ExcludeReview
            scan={scan}
            extraExcludes={excludes}
            onScan={(s, e) => {
              setScan(s);
              setExcludes(e);
            }}
            onNext={() => setStep("password")}
          />
        )}
        {view === "wizard" && step === "password" && (
          <PasswordSetup
            onNext={pw => {
              setPassword(pw);
              setStep("backup");
            }}
          />
        )}
        {view === "wizard" && (
          <>
            {step === "backup" && drive && scan && (
          <BackupProgress
            drive={drive}
            scan={scan}
            password={password}
            onDone={m => {
              setManifest(m);
              setStep("download");
            }}
          />
        )}
        {step === "download" && drive && os && (
          <DownloadProgress drive={drive} os={os} onDone={() => setStep("write")} />
        )}
        {step === "write" && drive && (
          <WriteProgress
            drive={drive}
            onDone={w => {
              setBootloaderWarning(w);
              setStep("done");
            }}
          />
        )}
        {step === "done" && drive && os && (
          <Done
            drive={drive}
            os={os}
            manifest={manifest}
            bootloaderWarning={bootloaderWarning}
            onRestart={restart}
          />
        )}
        {step === "restore-detect" && (
          <RestoreDetect
            onFound={(loc, pw) => {
              setBackupLoc(loc);
              setRestorePassword(pw);
              setStep("restore");
            }}
          />
        )}
        {step === "restore" && backupLoc && (
          <RestoreProgress
            loc={backupLoc}
            password={restorePassword}
            onDone={(s, dir) => {
              setSummary(s);
              setStagingDir(dir);
              setStep("apps");
            }}
          />
        )}
        {step === "apps" && (
          <AppPicker stagingDir={stagingDir} summary={summary} onRestart={restart} />
        )}
          </>
        )}
      </div>
    </div>
  );
}
