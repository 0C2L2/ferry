// Lemon Squeezy: merchant of record, so it also handles sales tax/VAT and
// sends the receipt. One product, one variant per size tier.

export interface LemonEnv {
  LEMONSQUEEZY_API_KEY?: string;
  LEMONSQUEEZY_STORE_ID?: string;
  LEMONSQUEEZY_WEBHOOK_SECRET?: string;
  LS_VARIANT_50GB?: string;
  LS_VARIANT_200GB?: string;
  LS_VARIANT_1TB?: string;
  CHECKOUT_REDIRECT_URL?: string;
}

const GB = 1024 ** 3;

export const TIERS = {
  "50gb": { label: "Up to 50 GB", maxBytes: 50 * GB, variant: "LS_VARIANT_50GB" },
  "200gb": { label: "Up to 200 GB", maxBytes: 200 * GB, variant: "LS_VARIANT_200GB" },
  "1tb": { label: "Up to 1 TB", maxBytes: 1024 * GB, variant: "LS_VARIANT_1TB" },
} as const;

export type Tier = keyof typeof TIERS;

export const isTier = (t: unknown): t is Tier => typeof t === "string" && t in TIERS;

export const variantFor = (env: LemonEnv, tier: Tier): string | undefined => env[TIERS[tier].variant];

/** A hosted checkout URL that carries the backup ID back in the webhook. */
export async function createCheckout(
  env: LemonEnv,
  opts: { email: string; backupId: string; variantId: string },
): Promise<string> {
  const res = await fetch("https://api.lemonsqueezy.com/v1/checkouts", {
    method: "POST",
    headers: {
      Accept: "application/vnd.api+json",
      "Content-Type": "application/vnd.api+json",
      Authorization: `Bearer ${env.LEMONSQUEEZY_API_KEY}`,
    },
    body: JSON.stringify({
      data: {
        type: "checkouts",
        attributes: {
          checkout_data: { email: opts.email, custom: { backup_id: opts.backupId } },
          ...(env.CHECKOUT_REDIRECT_URL ? { product_options: { redirect_url: env.CHECKOUT_REDIRECT_URL } } : {}),
        },
        relationships: {
          store: { data: { type: "stores", id: env.LEMONSQUEEZY_STORE_ID } },
          variant: { data: { type: "variants", id: opts.variantId } },
        },
      },
    }),
  });
  if (!res.ok) throw new Error(`Lemon Squeezy checkout failed: ${res.status} ${await res.text()}`);
  const body = (await res.json()) as { data: { attributes: { url: string } } };
  return body.data.attributes.url;
}

/** `X-Signature` is the hex HMAC-SHA256 of the raw body. crypto.subtle.verify
 *  compares in constant time. */
export async function verifyWebhook(secret: string, rawBody: string, signatureHex: string): Promise<boolean> {
  if (!/^[0-9a-f]{64}$/i.test(signatureHex)) return false;
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["verify"],
  );
  const sig = new Uint8Array(signatureHex.match(/../g)!.map(h => parseInt(h, 16)));
  return crypto.subtle.verify("HMAC", key, sig, new TextEncoder().encode(rawBody));
}
