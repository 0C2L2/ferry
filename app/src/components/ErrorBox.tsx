import { ShieldAlert } from "lucide-react";

/** Friendly message up front, raw backend text tucked into <details>. */
export function ErrorBox({
  message,
  detail,
  expanded,
}: {
  message: string;
  detail?: string;
  expanded?: boolean;
}) {
  return (
    <div className="bg-red-50 text-red-700 p-4 rounded-lg mb-6 flex items-start gap-3">
      <ShieldAlert className="w-5 h-5 shrink-0 mt-0.5" />
      <div className="min-w-0">
        <h3 className="font-medium">{message}</h3>
        {detail && (
          <details className="mt-2 text-sm" open={expanded}>
            <summary className="cursor-pointer">Technical details</summary>
            <p className="mt-1 break-words whitespace-pre-wrap">{detail}</p>
          </details>
        )}
      </div>
    </div>
  );
}
