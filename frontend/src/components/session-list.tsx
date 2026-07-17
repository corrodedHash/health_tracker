import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { listSessions, getWeightDetails, getRunningSummary } from "@/lib/api";
import type { ExerciseSession } from "@/types";

function formatDuration(ms: number): string {
  const secs = Math.floor(ms / 1000);
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  if (h > 0) return `${h}h ${m}m ${s}s`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

function formatPace(distanceM: number, durationMs: number): string {
  const km = distanceM / 1000;
  if (km <= 0) return "—";
  const secsPerKm = durationMs / 1000 / km;
  const m = Math.floor(secsPerKm / 60);
  const s = Math.round(secsPerKm % 60);
  return `${m}:${String(s).padStart(2, "0")} /km`;
}

function SessionRow({ session }: { session: ExerciseSession }) {
  // Load per-kind detail. These are skeleton placeholders and degrade
  // gracefully before the web crate lands.
  const weightQ = useQuery({
    queryKey: ["weight", session.id],
    queryFn: () => getWeightDetails(session.id),
    enabled: session.kind === "weight",
    retry: false,
  });
  const runningQ = useQuery({
    queryKey: ["running", session.id],
    queryFn: () => getRunningSummary(session.id),
    enabled: session.kind === "running",
    retry: false,
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex justify-between">
          <span className="capitalize">{session.kind}</span>
          <span className="text-sm font-normal text-muted-foreground">
            {session.startedAt.toLocaleString()}
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-1 text-sm">
        <div>
          <span className="text-muted-foreground">Duration:</span>{" "}
          {formatDuration(session.durationMs)}
        </div>
        {session.kind === "weight" && weightQ.data && (
          <div>
            <span className="text-muted-foreground">Exercise:</span>{" "}
            {weightQ.data.exercise_name} · {weightQ.data.weight_kg}kg ·{" "}
            {weightQ.data.sets}×{weightQ.data.reps}
          </div>
        )}
        {session.kind === "running" && runningQ.data && (
          <div>
            <span className="text-muted-foreground">Distance:</span>{" "}
            {(runningQ.data.distanceM / 1000).toFixed(2)} km ·{" "}
            <span className="text-muted-foreground">Pace:</span>{" "}
            {formatPace(runningQ.data.distanceM, session.durationMs)}
          </div>
        )}
        {session.notes && (
          <div className="text-muted-foreground">{session.notes}</div>
        )}
      </CardContent>
    </Card>
  );
}

const PAGE_SIZE = 10;

export function SessionList() {
  const [offset, setOffset] = useState(0);
  const [sessions, setSessions] = useState<ExerciseSession[]>([]);

  const q = useQuery({
    queryKey: ["sessions", offset],
    queryFn: () => listSessions(undefined, PAGE_SIZE, offset),
    retry: false,
  });

  const isFirstPage = offset === 0;

  if (isFirstPage && q.isLoading) {
    return <p className="text-sm text-muted-foreground">Loading sessions…</p>;
  }
  if (isFirstPage && (q.isError || !q.data)) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-muted-foreground">
          Couldn't load sessions (the web server may not be running yet).
        </p>
        <Button variant="outline" size="sm" onClick={() => q.refetch()}>
          Retry
        </Button>
      </div>
    );
  }

  const page = q.data ?? [];
  const all = isFirstPage ? page : [...sessions, ...page];
  if (isFirstPage) {
    if (all.length === 0) {
      return (
        <p className="text-sm text-muted-foreground">
          No sessions logged yet.
        </p>
      );
    }
  }

  const hasMore = page.length >= PAGE_SIZE;

  const handleLoadMore = () => {
    setSessions(all);
    setOffset(offset + PAGE_SIZE);
  };

  return (
    <div className="space-y-3">
      {all.map((s) => (
        <SessionRow key={s.id} session={s} />
      ))}
      {hasMore && (
        <div className="flex justify-center pt-2">
          <Button variant="outline" size="sm" onClick={handleLoadMore}>
            Load more
          </Button>
        </div>
      )}
    </div>
  );
}