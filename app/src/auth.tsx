import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

// Local-only accounts for this build. The core app needs no account at all;
// an account personalizes the app and reserves your identity for Cloud Backup
// (the planned paid feature). Passwords are stored as SHA-256 hashes, never
// plaintext. There is no server yet — everything stays on this PC.

export interface Account {
  name: string;
  email: string;
  passHash: string;
  createdAt: string;
}

interface AuthState {
  account: Account | null;
  signUp: (name: string, email: string, password: string) => Promise<string | null>;
  signIn: (email: string, password: string) => Promise<string | null>;
  signOut: () => void;
}

const AuthContext = createContext<AuthState | null>(null);

const ACCOUNTS_KEY = "ferry.accounts";
const SESSION_KEY = "ferry.session";

function loadAccounts(): Account[] {
  try {
    const raw = localStorage.getItem(ACCOUNTS_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? (parsed as Account[]) : [];
  } catch {
    return [];
  }
}

async function sha256Hex(text: string): Promise<string> {
  const bytes = new TextEncoder().encode(`ferry-local:${text}`);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map(b => b.toString(16).padStart(2, "0")).join("");
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [account, setAccount] = useState<Account | null>(null);

  useEffect(() => {
    const email = localStorage.getItem(SESSION_KEY);
    if (email) {
      setAccount(loadAccounts().find(a => a.email === email) ?? null);
    }
  }, []);

  async function signUp(name: string, email: string, password: string): Promise<string | null> {
    const cleanEmail = email.trim().toLowerCase();
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(cleanEmail)) return "Enter a valid email address.";
    if (password.length < 8) return "Password must be at least 8 characters.";
    if (!name.trim()) return "Enter your name.";
    const accounts = loadAccounts();
    if (accounts.some(a => a.email === cleanEmail)) {
      return "An account with this email already exists on this PC. Try signing in.";
    }
    const acc: Account = {
      name: name.trim(),
      email: cleanEmail,
      passHash: await sha256Hex(password),
      createdAt: new Date().toISOString(),
    };
    localStorage.setItem(ACCOUNTS_KEY, JSON.stringify([...accounts, acc]));
    localStorage.setItem(SESSION_KEY, acc.email);
    setAccount(acc);
    return null;
  }

  async function signIn(email: string, password: string): Promise<string | null> {
    const cleanEmail = email.trim().toLowerCase();
    const acc = loadAccounts().find(a => a.email === cleanEmail);
    if (!acc || (await sha256Hex(password)) !== acc.passHash) {
      return "Wrong email or password for this PC.";
    }
    localStorage.setItem(SESSION_KEY, acc.email);
    setAccount(acc);
    return null;
  }

  function signOut() {
    localStorage.removeItem(SESSION_KEY);
    setAccount(null);
  }

  return (
    <AuthContext.Provider value={{ account, signUp, signIn, signOut }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthState {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used inside AuthProvider");
  return ctx;
}
