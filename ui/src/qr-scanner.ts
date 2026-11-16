export type ScannerState = {
  phase: "idle" | "permission" | "scanning" | "stopping";
  permissionPending: boolean;
  blocked: boolean;
  message: string;
};
export type ScannerDriver = {
  checkPermissions(): Promise<string>;
  requestPermissions(): Promise<string>;
  scan(): Promise<{ content: string }>;
  cancel(): Promise<void>;
};
type Timers = { after(work: () => void, ms: number): unknown; clear(timer: unknown): void };
const timers: Timers = { after: (work, ms) => setTimeout(work, ms), clear: timer => clearTimeout(timer as ReturnType<typeof setTimeout>) };
const drivers = new WeakMap<ScannerDriver, { blocked: boolean; active: boolean; permissionPending: boolean }>();

export function createQrScanner(driver: ScannerDriver, changed: (state: ScannerState) => void, read: (text: string) => void, clock = timers) {
  let backend = drivers.get(driver);
  if (!backend) { backend = { blocked: false, active: false, permissionPending: false }; drivers.set(driver, backend); }
  const shared = backend;
  let state: ScannerState = { phase: "idle", permissionPending: false, blocked: false, message: "" };
  let generation = 0, started = false, disposed = false;
  let deadline: unknown;
  let stopping: Promise<void> | undefined;
  const update = (values: Partial<ScannerState>) => { state = { ...state, ...values }; if (!disposed) changed(state); };

  async function stop(message = "Сканирование отменено. Можно ввести адрес и код вручную.") {
    ++generation;
    clock.clear(deadline);
    update({ message });
    if (stopping) return stopping;
    if (!started) { update({ phase: "idle" }); return; }
    update({ phase: "stopping" });
    stopping = (async () => {
      let timeout: unknown;
      try {
        await Promise.race([Promise.resolve().then(() => driver.cancel()), new Promise<never>((_, reject) => {
          timeout = clock.after(() => reject(new Error("cancel_timeout")), 5000);
        })]);
      } catch {
        shared.blocked = true;
        update({ blocked: true, message: "Остановка камеры не подтверждена. Перезапустите приложение перед новым сканированием. Ручной ввод доступен." });
      } finally {
        clock.clear(timeout); started = false; shared.active = false; stopping = undefined; update({ phase: "idle" });
      }
    })();
    return stopping;
  }

  async function start() {
    if (disposed || state.phase !== "idle" || state.blocked || state.permissionPending) return;
    if (shared.blocked) {
      update({ blocked: true, message: "Остановка камеры не подтверждена. Перезапустите приложение перед новым сканированием. Ручной ввод доступен." }); return;
    }
    if (shared.active || shared.permissionPending) {
      update({ message: "Предыдущее сканирование ещё завершается. Повторите позже или введите данные вручную." }); return;
    }
    shared.permissionPending = true;
    const current = ++generation;
    let checkingPermission = true;
    update({ phase: "permission", permissionPending: true, message: "Разрешите камеру для чтения QR. Можно отменить и ввести данные вручную." });
    try {
      let permission = await driver.checkPermissions();
      if (current !== generation) return;
      if (permission !== "granted") permission = await driver.requestPermissions();
      if (current !== generation) return;
      if (permission !== "granted") {
        update({ phase: "idle", message: "Нет разрешения камеры. Введите адрес и код вручную или разрешите камеру в настройках Android и повторите." });
        return;
      }
      checkingPermission = false;
      shared.permissionPending = false;
      update({ permissionPending: false, phase: "scanning", message: "Наведите камеру на QR другого устройства. Можно отменить и ввести данные вручную." });
      started = true;
      shared.active = true;
      deadline = clock.after(() => void stop("QR не прочитан за 20 секунд. Камера или распознавание могут быть недоступны. Введите данные вручную или повторите, когда распознавание будет готово."), 20_000);
      const scanned = await driver.scan();
      if (current !== generation) return;
      const finished = stop("QR прочитан. Проверяем данные подключения.");
      const delivery = generation;
      await finished;
      if (!disposed && delivery === generation && !state.blocked) read(scanned.content);
    } catch {
      if (current === generation) await stop("Сканер недоступен. Приглашение не проверялось. Введите адрес и код вручную или повторите после готовности камеры и распознавания.");
    } finally {
      if (checkingPermission) { shared.permissionPending = false; update({ permissionPending: false }); }
    }
  }

  return { start, stop, dispose: () => { disposed = true; return stop(); } };
}
