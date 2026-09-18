import { useState } from "react";
import { useAuth } from "../auth";
import { ErrorBox } from "../components/ErrorBox";

/** Account settings only — profile, password, and account deletion.
 *  Backup/transfer choices are made fresh in the wizard every time. */
export function SettingsPage() {
  const { account, updateName, changePassword, deleteAccount } = useAuth();
  const [name, setName] = useState(account?.name ?? "");
  const [nameMsg, setNameMsg] = useState<string | null>(null);
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");
  const [pwMsg, setPwMsg] = useState<string | null>(null);
  const [pwOk, setPwOk] = useState(false);
  const [armingDelete, setArmingDelete] = useState(false);

  return (
    <div className="max-w-2xl">
      <div className="bg-white border border-gray-200 rounded-xl p-5 mb-4">
        <h3 className="font-medium mb-1">Display name</h3>
        <div className="flex gap-2">
          <input
            value={name}
            onChange={e => setName(e.target.value)}
            className="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm"
          />
          <button
            onClick={() => setNameMsg(updateName(name) ?? "Saved.")}
            className="px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700"
          >
            Save
          </button>
        </div>
        {nameMsg && <p className="text-sm text-gray-600 mt-2">{nameMsg}</p>}
      </div>

      <div className="bg-white border border-gray-200 rounded-xl p-5 mb-4">
        <h3 className="font-medium mb-1">Change password</h3>
        <label className="block text-sm text-gray-600 mt-3 mb-1">Current password</label>
        <input
          type="password"
          value={current}
          onChange={e => setCurrent(e.target.value)}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm"
        />
        <label className="block text-sm text-gray-600 mt-3 mb-1">New password (min 8)</label>
        <input
          type="password"
          value={next}
          onChange={e => setNext(e.target.value)}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm"
        />
        <label className="block text-sm text-gray-600 mt-3 mb-1">Confirm new password</label>
        <input
          type="password"
          value={confirm}
          onChange={e => setConfirm(e.target.value)}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm mb-3"
        />
        <button
          onClick={() => {
            setPwOk(false);
            if (next !== confirm) {
              setPwMsg("New passwords do not match.");
              return;
            }
            void changePassword(current, next).then(err => {
              if (err) {
                setPwMsg(err);
              } else {
                setPwMsg(null);
                setPwOk(true);
                setCurrent("");
                setNext("");
                setConfirm("");
              }
            });
          }}
          className="px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700"
        >
          Change password
        </button>
        {pwMsg && <div className="mt-3"><ErrorBox message={pwMsg} /></div>}
        {pwOk && <p className="text-sm text-green-700 mt-2">Password changed.</p>}
      </div>

      <div className="bg-white border border-red-200 rounded-xl p-5">
        <h3 className="font-medium mb-1 text-red-700">Delete account</h3>
        <p className="text-sm text-gray-500 mb-3">
          Removes your account from this PC and signs you out. USB backups are not touched.
        </p>
        {!armingDelete ? (
          <button
            onClick={() => setArmingDelete(true)}
            className="px-4 py-2 rounded-lg border border-red-300 text-sm text-red-700 hover:bg-red-50"
          >
            Delete my account…
          </button>
        ) : (
          <div className="flex gap-2 items-center flex-wrap">
            <span className="text-sm font-medium">Are you sure?</span>
            <button
              onClick={deleteAccount}
              className="px-4 py-2 rounded-lg bg-red-600 text-white text-sm font-medium hover:bg-red-700"
            >
              Yes, delete it
            </button>
            <button
              onClick={() => setArmingDelete(false)}
              className="px-4 py-2 rounded-lg border border-gray-300 text-sm"
            >
              Cancel
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
