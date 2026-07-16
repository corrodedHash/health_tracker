import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function LoginPage() {
  const beginOidc = () => {
    const resumeToken = crypto.randomUUID();
    localStorage.setItem(resumeToken, "");
    window.location.assign(`/auth/login?resume_token=${resumeToken}`);
  };

  return (
    <Card className="mx-auto max-w-sm">
      <CardHeader>
        <CardTitle>Sign in</CardTitle>
        <CardDescription>
          You'll be redirected to the OIDC provider, then back here.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Button onClick={beginOidc} className="w-full">
          Continue with OIDC
        </Button>
      </CardContent>
    </Card>
  );
}