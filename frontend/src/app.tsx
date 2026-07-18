import { useCallback, useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import axios from "axios";

import { ExerciseInput } from "@/components/exercise-input";
import { LoginPage } from "@/components/login-page";
import { SessionList } from "@/components/session-list";
import { RunningPaceChart } from "@/components/running-pace-chart";
import { RunningDistanceChart } from "@/components/running-distance-chart";
import { CalendarHeatmap } from "@/components/calendar-heatmap";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { checkAuth } from "@/lib/api";

function useResumeToken() {
  const qc = useQueryClient();
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const key = params.get("resume_token");
    if (!key) return;

    const data = localStorage.getItem(key);
    localStorage.removeItem(key);

    if (!data) {
      window.history.replaceState({}, "", "/");
      return;
    }

    const re = new URLSearchParams(data);
    const kind = re.get("kind") ?? "weight";
    const started_at = re.get("started_at") ?? new Date().toISOString();
    const duration_min = Number(re.get("duration_min") ?? "30");
    const notes = re.get("notes") || null;

    const payload = {
      kind,
      started_at,
      duration_secs: Math.max(1, duration_min) * 60,
      notes,
    };

    axios
      .post("/api/exercise-sessions", payload, {
        headers: { "Content-Type": "application/json" },
      })
      .then(() => {
        qc.invalidateQueries({ queryKey: ["sessions"] });
      })
      .catch((err) => {
        if (!axios.isAxiosError(err)) return;
        if (
          !err.response?.status ||
          ![401, 302].includes(err.response.status)
        ) {
          console.error("Resume-token replay failed:", err);
        } else {
          console.error("Unauthorized: resume token may be expired.");
        }
      })
      .finally(() => {
        window.history.replaceState({}, "", "/");
      });
  }, [qc]);
}

export default function App() {
  useResumeToken();
  const [activeTab, setActiveTab] = useState<"dashboard" | "graphs">("dashboard");

  const authQ = useQuery({
    queryKey: ["auth", "status"],
    queryFn: checkAuth,
    retry: false,
    refetchInterval: 60_000,
  });

  const logout = useMutation({
    mutationFn: () => axios.post("/auth/logout"),
    onSuccess: () => {
      authQ.refetch();
    },
    onError: () => {
      authQ.refetch();
    },
  });

  const logoutClick = useCallback(() => logout.mutate(), [logout]);

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
        <LoginPage />
      </div>
    );
  }

  const tabs: Array<"dashboard" | "graphs"> = ["dashboard", "graphs"];

  return (
    <div className="mx-auto max-w-4xl space-y-6 p-4 sm:p-6">
      <header className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold tracking-tight">Health Tracker</h1>
        <Button onClick={logoutClick} variant="outline" size="sm">
          Logout
        </Button>
      </header>

      <div className="flex gap-1 border-b pb-2">
        {tabs.map((tab) => (
          <Button
            key={tab}
            variant={activeTab === tab ? "default" : "outline"}
            size="sm"
            onClick={() => setActiveTab(tab)}
            className="capitalize"
          >
            {tab}
          </Button>
        ))}
      </div>

      {activeTab === "dashboard" ? (
        <>
          <ExerciseInput />

          <section className="space-y-3">
            <h2 className="text-lg font-semibold">Recent sessions</h2>
            <SessionList />
          </section>
        </>
      ) : (
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Running pace</CardTitle>
            </CardHeader>
            <CardContent>
              <RunningPaceChart />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Running distance</CardTitle>
            </CardHeader>
            <CardContent>
              <RunningDistanceChart />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Training heatmap</CardTitle>
            </CardHeader>
            <CardContent>
              <CalendarHeatmap />
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}