import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { issueToken, listTokens, revokeToken } from "@/lib/api";
import type { ApiToken, NewApiToken } from "@/types";

function formatDate(iso: string | null): string {
  if (!iso) return "never";
  return new Date(iso).toLocaleString();
}

function TokenRow({ token, onRevoke }: { token: ApiToken; onRevoke: () => void }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="flex items-center justify-between gap-3 rounded-md border border-input px-3 py-2">
      <div className="min-w-0">
        <div className="truncate font-medium">{token.label}</div>
        <div className="text-xs text-muted-foreground">
          Created {formatDate(token.created_at)} · Last used {formatDate(token.last_used_at)}
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => {
            navigator.clipboard.writeText(token.id);
            setCopied(true);
            setTimeout(() => setCopied(false), 1500);
          }}
          aria-label="Copy token ID"
        >
          <Copy />
          {copied ? "Copied" : "Copy ID"}
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="text-destructive hover:text-destructive"
          onClick={onRevoke}
          aria-label={`Revoke ${token.label}`}
        >
          <Trash2 />
          Revoke
        </Button>
      </div>
    </div>
  );
}

export function ApiTokenCard() {
  const qc = useQueryClient();
  const [label, setLabel] = useState("");
  const [fresh, setFresh] = useState<NewApiToken | null>(null);

  const tokensQ = useQuery({
    queryKey: ["tokens"],
    queryFn: listTokens,
    retry: false,
  });

  const issue = useMutation({
    mutationFn: () => issueToken(label.trim()),
    onSuccess: (token) => {
      setFresh(token);
      setLabel("");
      qc.invalidateQueries({ queryKey: ["tokens"] });
    },
  });

  const revoke = useMutation({
    mutationFn: (tokenId: string) => revokeToken(tokenId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["tokens"] }),
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>API tokens</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <p className="text-sm text-muted-foreground">
          Long-lived bearer tokens for machine clients (e.g. the Matrix bot). The
          token is only shown once — copy it right away.
        </p>

        <form
          className="flex items-end gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            if (label.trim()) issue.mutate();
          }}
        >
          <div className="flex-1 space-y-2">
            <Label htmlFor="token_label">Label</Label>
            <Input
              id="token_label"
              type="text"
              value={label}
              placeholder="matrix-bot"
              onChange={(e) => setLabel(e.target.value)}
            />
          </div>
          <Button type="submit" disabled={issue.isPending || !label.trim()}>
            {issue.isPending ? "Generating…" : "Generate token"}
          </Button>
        </form>

        {issue.isError && (
          <p className="text-sm text-destructive">Failed to generate token.</p>
        )}

        {fresh && (
          <div className="space-y-2 rounded-md border border-primary/40 bg-muted/50 px-3 py-3">
            <div className="flex items-center justify-between gap-3">
              <Label className="text-sm font-medium">New token ({fresh.label})</Label>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  navigator.clipboard.writeText(fresh.token);
                }}
              >
                <Copy />
                Copy
              </Button>
            </div>
            <code className="block break-all rounded bg-background px-2 py-1 font-mono text-xs">
              {fresh.token}
            </code>
            <p className="text-xs text-muted-foreground">
              Store this somewhere safe — it won't be shown again. Use it as the
              <code className="mx-1">api.token</code> / <code className="mx-1">HEALTH__API__TOKEN</code>{" "}
              for the Matrix bot.
            </p>
          </div>
        )}

        <div className="space-y-2">
          {tokensQ.isLoading && (
            <p className="text-sm text-muted-foreground">Loading tokens…</p>
          )}
          {tokensQ.data && tokensQ.data.length === 0 && (
            <p className="text-sm text-muted-foreground">No tokens yet.</p>
          )}
          {tokensQ.data?.map((token) => (
            <TokenRow
              key={token.id}
              token={token}
              onRevoke={() => revoke.mutate(token.id)}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
