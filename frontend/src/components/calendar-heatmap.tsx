import ReactECharts from "echarts-for-react";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import dayjs from "dayjs";
import { listSessions } from "@/lib/api";

export function CalendarHeatmap() {
  const sessionsQ = useQuery({
    queryKey: ["sessions", "all"],
    queryFn: () => listSessions(),
    retry: false,
  });

  const option = useMemo(() => {
    const sessions = sessionsQ.data ?? [];

    const dayMap = new Map<string, number>();
    for (const s of sessions) {
      if (s.quality == null) continue;
      const key = dayjs(s.startedAt).format("YYYY-MM-DD");
      const current = dayMap.get(key);
      dayMap.set(key, current != null ? Math.max(current, s.quality) : s.quality);
    }

    const data = Array.from(dayMap.entries()).map(([date, quality]) => [date, quality]);

    if (data.length === 0) return null;

    const years = [...new Set(data.map((d) => (d[0] as string).slice(0, 4)))].sort();
    const range = years.length === 1 ? years[0] : [years[0], years[years.length - 1]];

    return {
      visualMap: {
        min: 1,
        max: 5,
        calculable: true,
        orient: "horizontal",
        left: "center",
        inRange: { color: ["#ebedf0", "#9be9a8", "#40c463", "#30a14e", "#216e39"] },
      },
      calendar: {
        range,
        cellSize: ["auto", 15],
        dayLabel: { show: true },
        monthLabel: { show: true },
        splitLine: { show: true },
      },
      series: [
        {
          type: "heatmap",
          coordinateSystem: "calendar",
          data,
        },
      ],
    };
  }, [sessionsQ.data]);

  if (sessionsQ.isLoading) {
    return <p className="text-sm text-muted-foreground">Loading session data…</p>;
  }

  if (!option) {
    return (
      <p className="text-sm text-muted-foreground">
        No sessions with quality ratings yet — log one with a quality score to see the heatmap.
      </p>
    );
  }

  return <ReactECharts option={option} style={{ height: 200, width: "100%" }} />;
}
