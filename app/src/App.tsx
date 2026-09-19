import { useState } from "react";
import { ArrowLeft, CircleHelp, RotateCcw, Tags, UserRound, Usb } from "lucide-react";
import { api } from "./api";
import { AuthProvider, useAuth } from "./auth";
import { Welcome } from "./pages/Welcome";
import { DriveSelect } from "./pages/DriveSelect";
import { PartitionUsb } from "./pages/PartitionUsb";
import { OSSelect } from "./pages/OSSelect";
import { MigrationPlan } from "./pages/MigrationPlan";
import { ExcludeReview } from "./pages/ExcludeReview";
import { PasswordSetup } from "./pages/PasswordSetup";
import { BackupProgress } from "./pages/BackupProgress";
import { DownloadProgress } from "./pages/DownloadProgress";
import { BootloaderProgress } from "./pages/BootloaderProgress";
import { Done } from "./pages/Done";
import { RestoreDetect } from "./pages/RestoreDetect";
import { RestoreProgress } from "./pages/RestoreProgress";
import { AppPicker } from "./pages/AppPicker";
import { Pricing } from "./pages/Pricing";
import { Faq } from "./pages/Faq";
import { Account, type AccountTab } from "./pages/Account";
import { CloudUpload } from "./pages/CloudUpload";
import { recordBackup, recordRestore } from "./history";
import type {
  BackupLocation,
  DriveInfo,
  Manifest,
  OsSource,
  RestoreSummary,
  ScanResult,
  UsbLayout,
} from "./types";

type Step =
  | "welcome"
  | "drives"
  | "partition"
  | "os"
  | "plan"
  | "excludes"
  | "password"
  | "backup"
  | "cloud"
  | "download"
  | "bootloader"
  | "done"
  | "restore-detect"
  | "restore"
  | "apps";

const BACKUP_BACK: Partial<Record<Step, Step>> = {
  drives: "welcome",
  partition: "drives",
  os: "partition",
  plan: "os",
  excludes: "plan",
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
  const [accountTab, setAccountTab] = useState<AccountTab>("profile");
  const [step, setStep] = useState<Step>("welcome");
  const [drive, setDrive] = useState<DriveInfo | null>(null);
  const [usbLayout, setUsbLayout] = useState<UsbLayout | null>(null);
  const [os, setOs] = useState<OsSource | null>(null);
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [excludes, setExcludes] = useState<string[]>([]);
  const [roots, setRoots] = useState<string[]>([]);
  const [password, setPassword] = useState("");
  const [manifest, setManifest] = useState<Manifest | null>(null);
  const [isoFilename, setIsoFilename] = useState<string | null>(null);
  const [bootloaderWarning, setBootloaderWarning] = useState<string | null>(null);
  const [backupLoc, setBackupLoc] = useState<BackupLocation | null>(null);
  const [restorePassword, setRestorePassword] = useState("");
  const [cloudBackupId, setCloudBackupId] = useState<string | null>(null);
  const [summary, setSummary] = useState<RestoreSummary | null>(null);
  const [stagingDir, setStagingDir] = useState<string | null>(null);

  function restart() {
    setStep("welcome");
    setDrive(null);
    setUsbLayout(null);
    setOs(null);
    setScan(null);
    setExcludes([]);
    setRoots([]);
    setPassword("");
    setManifest(null);
    setIsoFilename(null);
    setBootloaderWarning(null);
    setBackupLoc(null);
    setRestorePassword("");
    setCloudBackupId(null);
    setSummary(null);
    setStagingDir(null);
  }

  const back = view === "wizard" ? BACKUP_BACK[step] : undefined;
  const NAV: { id: View; label: string; icon: typeof Usb }[] = [
    { id: "wizard", label: "Migrate", icon: Usb },
    { id: "pricing", label: "Pricing", icon: Tags },
    { id: "faq", label: "FAQ", icon: CircleHelp },
    { id: "account", label: account ? account.name.split(" ")[0] : "Sign in", icon: UserRound },
  ];

  const sideNav = (
    <>
      {NAV.map(item => {
        const Icon = item.icon;
        const active = view === item.id;
        return (
          <button
            key={item.id}
            onClick={() => {
              if (item.id === "account") setAccountTab("profile");
              setView(item.id);
            }}
            className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-lg text-sm transition ${
              active
                ? "bg-brand-50 text-brand-700 font-semibold"
                : "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
            }`}
          >
            <Icon className="w-5 h-5 shrink-0" />
            <span className="truncate">{item.label}</span>
          </button>
        );
      })}
    </>
  );

  return (
    <div className="h-screen w-full bg-gray-50 text-gray-900 flex overflow-hidden">
      {/* Left sidebar (desktop) */}
      <aside className="hidden md:flex w-60 shrink-0 flex-col bg-white border-r border-gray-200 py-6 px-4">
        <div className="px-2 mb-8">
          <h1 className="text-2xl font-bold">Ferry</h1>
          <p className="text-gray-500 text-xs mt-0.5">Backup & OS Reinstallation Assistant</p>
        </div>
        <nav className="space-y-1 flex-1">
          {sideNav}
        </nav>
        <div className="pt-4 border-t border-gray-100 space-y-1">
          {back && (
            <button
              onClick={() => setStep(back)}
              className="w-full flex items-center gap-3 px-4 py-2 text-sm text-gray-500 hover:text-gray-900"
            >
              <ArrowLeft className="w-5 h-5 shrink-0" /> Back
            </button>
          )}
          {view === "wizard" && step !== "welcome" && (
            <button
              onClick={restart}
              className="w-full flex items-center gap-3 px-4 py-2 text-sm text-gray-500 hover:text-gray-900"
            >
              <RotateCcw className="w-5 h-5 shrink-0" /> Start over
            </button>
          )}
        </div>
      </aside>

      <div className="flex-1 flex flex-col min-w-0">
        {/* Compact top bar (narrow screens only) */}
        <header className="md:hidden flex items-center gap-1 bg-white border-b border-gray-200 px-3 py-2 overflow-x-auto shrink-0">
          <span className="font-bold mr-2 shrink-0">Ferry</span>
          {NAV.map(item => (
            <button
              key={item.id}
              onClick={() => {
                if (item.id === "account") setAccountTab("profile");
                setView(item.id);
              }}
              className={`text-sm px-2 py-1 rounded whitespace-nowrap ${
                view === item.id ? "text-brand-700 font-semibold" : "text-gray-500"
              }`}
            >
              {item.label}
            </button>
          ))}
        </header>

        {/* Scrollable content */}
        <main className="flex-1 overflow-y-auto p-4 sm:p-8">
          <div className="max-w-3xl mx-auto pb-12">
        {view === "pricing" && <Pricing onAccount={() => setView("account")} />}
        {view === "faq" && <Faq />}
        {view === "account" && <Account tab={accountTab} onTabChange={setAccountTab} />}
        {view === "wizard" && step === "welcome" && (
          <Welcome onBackup={() => setStep("drives")} onRestore={() => setStep("restore-detect")} />
        )}
        {view === "wizard" && step === "drives" && (
          <DriveSelect selected={drive} onSelect={setDrive} onNext={() => setStep("partition")} />
        )}
        {view === "wizard" && step === "partition" && drive && (
          <PartitionUsb
            drive={drive}
            onDone={layout => {
              setUsbLayout(layout);
              setStep("os");
            }}
          />
        )}
        {view === "wizard" && step === "os" && (
          <OSSelect selected={os} onSelect={setOs} onNext={() => setStep("plan")} />
        )}
        {view === "wizard" && step === "plan" && os && (
          <MigrationPlan
            os={os}
            onDone={r => {
              setRoots(r);
              setScan(null);
              setExcludes([]);
              setStep("excludes");
            }}
          />
        )}
        {view === "wizard" && step === "excludes" && (
          <ExcludeReview
            scan={scan}
            extraExcludes={excludes}
            roots={roots}
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
            {step === "backup" && usbLayout && scan && (
          <BackupProgress
            dataRoot={usbLayout.data_letter}
            scan={scan}
            password={password}
            onDone={m => {
              setManifest(m);
              if (usbLayout && os) {
                recordBackup({
                  drive: usbLayout.data_letter,
                  os: os.label,
                  files: m.files.length,
                  bytes: m.files.reduce((sum, f) => sum + f.size_bytes, 0),
                });
              }
              setStep("cloud"); // offer Extra Careful cloud upload first
            }}
          />
        )}
        {step === "cloud" && usbLayout && (
          <CloudUpload
            dataRoot={usbLayout.data_letter}
            onDone={() => setStep("download")}
            onSkip={() => setStep("download")}
          />
        )}
        {step === "download" && usbLayout && os && (
          <DownloadProgress
            dataRoot={usbLayout.data_letter}
            os={os}
            onDone={filename => {
              setIsoFilename(filename);
              setStep("bootloader");
            }}
          />
        )}
        {step === "bootloader" && usbLayout && isoFilename && (
          <BootloaderProgress
            layout={usbLayout}
            isoFilename={isoFilename}
            onDone={w => {
              setBootloaderWarning(w);
              setStep("done");
            }}
          />
        )}
        {step === "done" && usbLayout && os && (
          <Done
            dataRoot={usbLayout.data_letter}
            os={os}
            manifest={manifest}
            bootloaderWarning={bootloaderWarning}
            onRestart={restart}
          />
        )}
        {step === "restore-detect" && (
          <RestoreDetect
            onFound={(loc, pw, cloudId) => {
              setBackupLoc(loc);
              setRestorePassword(pw);
              setCloudBackupId(cloudId ?? null);
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
              recordRestore({ restored: s.restored, skipped: s.skipped, wifi: s.wifi_restored });
              // Cleans up Ferry's cloud copy now that it's safely restored
              // locally — best-effort, never blocks the user from continuing.
              if (cloudBackupId) {
                api.deleteCloudBackup(cloudBackupId).catch(() => {});
              }
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
        </main>
      </div>
    </div>
  );
}
