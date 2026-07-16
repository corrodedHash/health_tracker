import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { createSession } from "@/lib/api";
import type { ExerciseKind } from "@/types";

const KINDS: ExerciseKind[] = ["weight", "core", "running"];

export function ExerciseInput() {
  const qc = useQueryClient();
  const [kind, setKind] = useState<ExerciseKind>("weight");
  const [startedAt, setStartedAt] = useState(() => new Date().toISOString().slice(0, 16));
  const [durationMin, setDurationMin] = useState("30");
  const [notes, setNotes] = useState("");

  const startOidcResume = (formData: URLSearchParams) => {
    const resumeToken = crypto.randomUUID();
    localStorage.setItem(resumeToken, formData.toString());
    window.location.assign(`/auth/login?resume_token=${resumeToken}`);
  };

  const mutation = useMutation({
    mutationFn: async () => {
      const body = {
        kind,
        startedAt: new Date(startedAt),
        durationMs: Math.max(1, Number(durationMin)) * 60_000,
        notes: notes.trim() || null,
      };
      return createSession(body);
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

  const onKindChange = (k: ExerciseKind) => {
    document.querySelectorAll<HTMLButtonElement>("[data-kind]").forEach((el) => {
      el.dataset.kind && el.setAttribute("data-active", String(el.dataset.kind === k));
    });
    setKind(k);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Log an exercise</CardTitle>
      </CardHeader>
      <CardContent>
        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            mutation.mutate();
          }}
        >
          <div className="space-y-2">
            <Label>Kind</Label>
            <div className="flex gap-2">
              {KINDS.map((k) => (
                <Button
                  key={k}
                  type="button"
                  variant={kind === k ? "default" : "outline"}
                  size="sm"
                  onClick={() => onKindChange(k)}
                  data-kind={k}
                  data-active={String(kind === k)}
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
            <Label htmlFor="notes">Notes</Label>
            <Input
              id="notes"
              type="text"
              value={notes}
              placeholder="optional"
              onChange={(e) => setNotes(e.target.value)}
            />
          </div>

          <Button type="submit" disabled={mutation.isPending}>
            {mutation.isPending ? "Saving…" : "Add session"}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}