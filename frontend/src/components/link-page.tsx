import { useMutation, useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { checkAuth, confirmLink } from "@/lib/api";

function beginLogin(code: string) {
  const resumeToken = crypto.randomUUID();
  localStorage.setItem(resumeToken, JSON.stringify({ link: `/link?code=${code}` }));
  window.location.assign(`/api/auth/login?resume_token=${resumeToken}`);
}

export function LinkPage() {
  const code = new URLSearchParams(window.location.search).get("code");

  const authQ = useQuery({
    queryKey: ["auth", "status"],
    queryFn: checkAuth,
    retry: false,
  });

  const confirm = useMutation({
    mutationFn: () => {
      if (!code) throw new Error("missing link code");
      return confirmLink(code);
    },
  });

  if (!code) {
    return (
      <div className="mx-auto max-w-4xl space-y-6 p-4 sm:p-6">
        <header className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold tracking-tight">Health Tracker</h1>
        </header>
        <Card className="mx-auto max-w-sm">
          <CardHeader>
            <CardTitle>Link your account</CardTitle>
            <CardDescription>Missing or invalid link code.</CardDescription>
          </CardHeader>
        </Card>
      </div>
    );
  }

  if (authQ.isPending) {
    return (
      <div className="mx-auto max-w-4xl space-y-6 p-4 sm:p-6">
        <header className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold tracking-tight">Health Tracker</h1>
        </header>
        <p className="text-sm text-muted-foreground">Checking authentication…</p>
      </div>
    );
  }

  if (!authQ.data) {
    return (
      <div className="mx-auto max-w-4xl space-y-6 p-4 sm:p-6">
        <header className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold tracking-tight">Health Tracker</h1>
        </header>
        <Card className="mx-auto max-w-sm">
          <CardHeader>
            <CardTitle>Link your account</CardTitle>
            <CardDescription>
              Sign in to let the Matrix bot add GPX files to your account.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={() => beginLogin(code)} className="w-full">
              Sign in to continue
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-4xl space-y-6 p-4 sm:p-6">
      <header className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold tracking-tight">Health Tracker</h1>
      </header>
      <Card className="mx-auto max-w-sm">
        <CardHeader>
          <CardTitle>Link your account</CardTitle>
          <CardDescription>
            Confirm that the Matrix bot may upload GPX files to your account.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {confirm.isSuccess ? (
            <p className="text-sm text-muted-foreground">
              Linked! You can now send a <code className="font-mono">.gpx</code> file to the bot.
            </p>
          ) : (
            <>
              {confirm.isError && (
                <p className="text-sm text-destructive">
                  Failed to link. The link may be expired or already used — try again from the
                  chat.
                </p>
              )}
              <Button
                onClick={() => confirm.mutate()}
                disabled={confirm.isPending}
                className="w-full"
              >
                {confirm.isPending ? "Linking…" : "Confirm link"}
              </Button>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
