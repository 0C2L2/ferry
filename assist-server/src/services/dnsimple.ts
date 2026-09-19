import { DNSimple } from "dnsimple";
import { customAlphabet } from "nanoid";

const nanoid = customAlphabet("abcdefghijklmnopqrstuvwxyz0123456789", 6);

export interface ShareLink {
  subdomain: string;
  url: string;
}

/**
 * Provisions a fresh subdomain for a Cloud Backup status page, live, via the
 * DNSimple API. TLS is served by a wildcard certificate for
 * *.{DNSIMPLE_DOMAIN} provisioned once ahead of time (see README.md) —
 * issuing a fresh Let's Encrypt cert per subdomain is a multi-minute DNS-01
 * challenge and isn't demo-friendly, so we don't do that per request.
 */
export async function createShareLink(backupId: string): Promise<ShareLink> {
  const accessToken = process.env.DNSIMPLE_API_TOKEN;
  const accountId = process.env.DNSIMPLE_ACCOUNT_ID;
  const domain = process.env.DNSIMPLE_DOMAIN;
  const target = process.env.PUBLIC_HOST;
  if (!accessToken || !accountId || !domain || !target) {
    throw new Error(
      "DNSimple is not configured (need DNSIMPLE_API_TOKEN, DNSIMPLE_ACCOUNT_ID, DNSIMPLE_DOMAIN, PUBLIC_HOST)",
    );
  }

  const client = new DNSimple({ accessToken });
  const subdomain = `backup-${nanoid()}`;
  const recordType = process.env.DNSIMPLE_RECORD_TYPE === "CNAME" ? "CNAME" : "A";

  await client.zones.createZoneRecord(Number(accountId), domain, {
    type: recordType,
    name: subdomain,
    content: target,
    ttl: 300,
  });

  return { subdomain, url: `https://${subdomain}.${domain}/status/${backupId}` };
}
