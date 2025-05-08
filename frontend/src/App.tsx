import { on, createEffect, onMount, Show, createSignal } from "solid-js";
import { Button } from "./components/ui/button";
import {
  Card,
  CardHeader,
  CardContent,
  CardTitle,
  CardDescription,
} from "./components/ui/card";
import {
  Tabs,
  TabsTrigger,
  TabsList,
  TabsContent,
  TabsIndicator,
} from "./components/ui/tabs";
import { Transition } from "solid-transition-group";
import { useToggle, useToNumber, useWebSocket } from "solidjs-use";
import { useClamp } from "@solidjs-use/math";
import WifiSlashIcon from "~icons/uil/wifi-slash";
import WifiIcon from "~icons/uil/wifi";
// used to $fetch because of nuxt
import { ofetch as $fetch } from "ofetch";
import {
  Chart,
  Tooltip,
  Colors,
  CategoryScale,
  LinearScale,
  LineController,
  PointElement,
  LineElement,
} from "chart.js";
import { TextField, TextFieldRoot } from "./components/ui/textfield";

function roundToTens(num: number) {
  return Math.round(num / 10) * 10;
}

function reset() {
  return $fetch("/api/reset", { method: "post" });
}
function turnServo() {
  return $fetch("/api/trigger/turn", { method: "post" });
}
function initiateShot(angle: number) {
  return $fetch("/api/trigger/initiate", { method: "post", body: { angle } });
}

function parseWsMessage(
  data: any,
): { kind: string; data: Record<string, any> } | undefined {
  try {
    const json = JSON.parse(data);
    if ("kind" in json) return json;
  } catch {
    /* nothing */
  }

  return undefined;
}

function App() {
  let chartEl!: HTMLCanvasElement;

  const ws = useWebSocket("/api/ws", { autoReconnect: true });

  const { addRpmValue } = useRpmChart(() => chartEl, { items: 10 });
  const [initiated, setInitiated] = createSignal(false);
  const [selectedMode, setSelectedMode] = createSignal("manual");

  const [_angleValue, _setAngleValue] = createSignal(45);
  const [angleValue, setAngleValue] = useClamp(
    [useToNumber(_angleValue, { nanToZero: true }), _setAngleValue],
    0,
    360,
  );
  const [doFormatAngle, toggleFormatAngle] = useToggle(true);
  const { format: formatAngle } = Intl.NumberFormat(undefined, {
    unit: "degree",
    style: "unit",
    unitDisplay: "narrow",
  });
  const formattedAngle = () => {
    return doFormatAngle()
      ? formatAngle(angleValue())
      : angleValue().toString();
  };

  createEffect(
    on(
      () => ws.data(),
      (wsData) => {
        console.info(wsData);
        const data = parseWsMessage(wsData);
        if (!data) return;

        if (data.kind == "RPM_UPDATE") {
          addRpmValue(data.data.rpm);
        }
        if (data.kind == "INITIATE_UPDATE") {
          setInitiated(data.data.value);
        }
      },
    ),
  );

  return (
    <div>
      <div class="bg-muted h-dvh md:grid place-center">
        <div class="md:px-8 m-auto max-w-[1320px]">
          <section>
            <Card class="p-4 md:p-8 justify-cente r gap-6 rounded-lg flex flex-col md:grid md:grid-cols-[1fr_2fr] place-items-stretch">
              <div class="flex flex-col gap-6 md:w-[350px]">
                <Card>
                  <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle class="font-medium">Verbindungsstatus</CardTitle>
                    <div class="size-5 *:size-5">
                      <Transition name="fade">
                        <Show
                          when={ws.status() == "OPEN"}
                          children={
                            <WifiIcon class="absolute text-emerald-500" />
                          }
                          fallback={
                            <WifiSlashIcon class="absolute text-rose-500" />
                          }
                        />
                      </Transition>
                    </div>
                  </CardHeader>
                  <CardContent>
                    <div class="text-2xl font-bold">
                      <Transition name="fade">
                        <Show
                          when={ws.status() == "OPEN"}
                          children={<div>Verbunden</div>}
                          fallback={<div>Verbindungsfehler</div>}
                        />
                      </Transition>
                    </div>
                    <p class="text-xs text-muted-foreground">
                      <Transition name="fade">
                        <Show
                          when={ws.status() != "OPEN"}
                          children={<div>Konnte keine Verbindung aufbauen</div>}
                        />
                      </Transition>
                    </p>
                  </CardContent>
                </Card>
                <Card class="flex-1">
                  <CardHeader>
                    <CardTitle>Modus Konfigurieren</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <Tabs onChange={(tab) => setSelectedMode(tab)}>
                      <TabsList class="grid w-full grid-cols-2">
                        <TabsTrigger value="manual">Manuell</TabsTrigger>
                        <TabsTrigger value="automatic">Winkel</TabsTrigger>
                        <TabsIndicator />
                      </TabsList>
                      <TabsContent value="manual" class="flex flex-col gap-2">
                        <CardDescription class="text-balance">
                          Manueller Abschuss mittels Knopfdruck
                        </CardDescription>
                        <Button on:click={reset} variant="outline">
                          Reset
                        </Button>
                        <Button on:click={turnServo}>Servo Drehen</Button>
                      </TabsContent>
                      <TabsContent
                        value="automatic"
                        class="flex flex-col gap-2"
                      >
                        <CardDescription class="text-balance">
                          Abschuss mit auf bestimmten Winkel
                        </CardDescription>
                        <div class="grid grid-cols-2 gap-2">
                          <TextFieldRoot
                            class="w-full max-w-xs"
                            value={formattedAngle()}
                            onFocusIn={() => toggleFormatAngle(false)}
                            onFocusOut={() => toggleFormatAngle(true)}
                            onChange={(v) => {
                              setAngleValue(parseInt(v));
                            }}
                          >
                            <TextField
                              type="text"
                              placeholder="Abschusswinkel"
                            />
                          </TextFieldRoot>
                          <Button on:click={turnServo} variant="outline">
                            Servo Drehen
                          </Button>
                        </div>
                        <Button on:click={() => initiateShot(angleValue())}>
                          {initiated()
                            ? "am schießen... (abbrechen)"
                            : "Abschuss!"}
                        </Button>
                      </TabsContent>
                    </Tabs>
                  </CardContent>
                </Card>
              </div>

              <div>
                <div class="grid h-full min-h-[410px] transition-all grid-rows-[1fr_0fr] gap-0">
                  <Card class="overflow-hidden flex flex-col">
                    <CardHeader>
                      <CardTitle>RPM</CardTitle>
                    </CardHeader>
                    <CardContent class="relative overflow-hidden flex-1">
                      <canvas class="max-h-full absolute" ref={chartEl} />
                    </CardContent>
                  </Card>
                  <div class="overflow-hidden">
                    <Card class="overflow-hidden h-full">
                      <CardHeader>
                        <CardTitle></CardTitle>
                      </CardHeader>
                      <CardContent class="pb-3"></CardContent>
                    </Card>
                  </div>
                </div>
              </div>
            </Card>
          </section>
        </div>
      </div>
    </div>
  );
}

export default App;

function useRpmChart(
  chartEl: () => HTMLCanvasElement,
  options = { items: 10 },
) {
  let chart: Chart<"line">;
  const data = Array.from({ length: options.items }, () => 0);
  const labels = Array.from({ length: options.items }, (_, i) => i + 1);
  let i = options.items;

  onMount(() => {
    Chart.register(
      Tooltip,
      Colors,
      CategoryScale,
      LinearScale,
      LineController,
      PointElement,
      LineElement,
    );
    chart = new Chart(chartEl(), {
      type: "line",
      data: {
        labels: labels,
        datasets: [
          {
            label: "RPM",
            data: data,
            cubicInterpolationMode: "monotone",
            tension: 0.4,
          },
        ],
      },
      options: {
        plugins: {
          legend: {
            display: false,
          },
        },
        scales: {
          x: {
            display: false,
          },
          y: {
            min: 0,
            max: 60,
          },
        },
        responsive: true,
        maintainAspectRatio: false,
      },
    });
  });

  function addRpmValue(value: number) {
    chart.data.labels?.push(i);
    chart.options.scales!.y!.max = Math.max(
      roundToTens(value + 5),
      chart.options.scales!.y!.max as number,
    );

    data.push(value);
    chart.update();

    // shift later to avoid weird bottom-up transition
    data.shift();
    chart.data.labels?.shift();
    chart.update();
  }

  // debug
  // useIntervalFn(() => {
  //   addRpmValue(Math.random() * 100);
  // }, 1000);

  return { addRpmValue };
}
