import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  createCoreDetails,
  createRunningDetails,
  createSession,
  createWeightDetails,
} from "@/lib/api";
import type { ExerciseKind } from "@/types";

const KINDS: ExerciseKind[] = ["weight", "core", "running", "custom"];

export function ExerciseInput() {
  const qc = useQueryClient();
  const [kind, setKind] = useState<ExerciseKind>("weight");
  const [startedAt, setStartedAt] = useState(() => new Date().toISOString().slice(0, 16));
  const [durationMin, setDurationMin] = useState("30");
  const [quality, setQuality] = useState("");
  const [notes, setNotes] = useState("");

  const [exerciseName, setExerciseName] = useState("");
  const [weightKg, setWeightKg] = useState("");
  const [sets, setSets] = useState("");
  const [reps, setReps] = useState("");
  const [distanceM, setDistanceM] = useState("");

  const startOidcResume = (formData: URLSearchParams) => {
    const resumeToken = crypto.randomUUID();
    localStorage.setItem(resumeToken, formData.toString());
    window.location.assign(`/auth/login?resume_token=${resumeToken}`);
  };

  const mutation = useMutation({
    mutationFn: async () => {
      const durationMs = Math.max(1, Number(durationMin)) * 60_000;
      const session = await createSession({
        kind,
        startedAt: new Date(startedAt),
        durationMs,
        notes: notes.trim() || null,
        quality: quality.trim() ? Number(quality) : null,
      });

      if (kind === "weight") {
        await createWeightDetails(session.id, {
          exercise_name: exerciseName.trim(),
          weight_kg: Number(weightKg),
          sets: Number(sets),
          reps: Number(reps),
          quality: null,
        });
      } else if (kind === "core") {
        await createCoreDetails(session.id, {
          exercise_name: exerciseName.trim(),
          duration_secs: durationMs / 1000,
          quality: null,
        });
      } else if (kind === "running") {
        await createRunningDetails(session.id, {
          distance_m: Number(distanceM),
        });
      }

      return session;
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ["sessions"] }),
    onError: (err) => {
      const status = (err as { response?: { status?: number } }).response?.status;
      if (status && [401, 302].includes(status)) {
        const formData = new URLSearchParams({
          kind,
          started_at: startedAt,
          duration_min: durationMin,
          notes,
        });
        startOidcResume(formData);
        return;
      }
      console.error("Failed to create session", err);
    },
  });

  return (
    <form
      className="space-y-4"
      onSubmit={(e) => {
        e.preventDefault();
        mutation.mutate();
      }}
    >
      {/* Card 1: Common fields for every exercise type */}
      <Card>
        <CardHeader>
          <CardTitle>Session</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Type</Label>
            <div className="flex gap-2 flex-wrap">
              {KINDS.map((k) => (
                <Button
                  key={k}
                  type="button"
                  variant={kind === k ? "default" : "outline"}
                  size="sm"
                  onClick={() => setKind(k)}
                >
                  <span className="capitalize">{k}</span>
                </Button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="started_at">Start</Label>
              <Input
                id="started_at"
                type="datetime-local"
                value={startedAt}
                onChange={(e) => setStartedAt(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="duration_min">Duration (min)</Label>
              <Input
                id="duration_min"
                type="number"
                min="1"
                value={durationMin}
                onChange={(e) => setDurationMin(e.target.value)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="quality">Quality (1–10, optional)</Label>
            <Input
              id="quality"
              type="number"
              min="1"
              max="10"
              value={quality}
              placeholder="optional"
              onChange={(e) => setQuality(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="notes">Notes</Label>
            <Input
              id="notes"
              type="text"
              value={notes}
              placeholder={
                kind === "custom" ? "Describe what you did" : "optional"
              }
              onChange={(e) => setNotes(e.target.value)}
            />
          </div>
        </CardContent>
      </Card>

      {/* Card 2: Type-specific fields (hidden for custom) */}
      {kind !== "custom" && (
        <Card>
          <CardHeader>
            <CardTitle className="capitalize">{kind} details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {(kind === "weight" || kind === "core") && (
              <div className="space-y-2">
                <Label htmlFor="exercise_name">Exercise name</Label>
                <Input
                  id="exercise_name"
                  type="text"
                  value={exerciseName}
                  placeholder="e.g. bench press"
                  onChange={(e) => setExerciseName(e.target.value)}
                />
              </div>
            )}

            {kind === "weight" && (
              <div className="grid grid-cols-3 gap-3">
                <div className="space-y-2">
                  <Label htmlFor="weight_kg">Weight (kg)</Label>
                  <Input
                    id="weight_kg"
                    type="number"
                    min="0"
                    step="0.5"
                    value={weightKg}
                    onChange={(e) => setWeightKg(e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="sets">Sets</Label>
                  <Input
                    id="sets"
                    type="number"
                    min="1"
                    value={sets}
                    onChange={(e) => setSets(e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="reps">Reps</Label>
                  <Input
                    id="reps"
                    type="number"
                    min="1"
                    value={reps}
                    onChange={(e) => setReps(e.target.value)}
                  />
                </div>
              </div>
            )}

            {kind === "running" && (
              <div className="space-y-2">
                <Label htmlFor="distance_m">Distance (m)</Label>
                <Input
                  id="distance_m"
                  type="number"
                  min="0"
                  step="1"
                  value={distanceM}
                  onChange={(e) => setDistanceM(e.target.value)}
                />
              </div>
            )}
          </CardContent>
        </Card>
      )}

      <Button type="submit" disabled={mutation.isPending}>
        {mutation.isPending ? "Saving…" : "Add session"}
      </Button>
    </form>
  );
}