export interface Toast {
  id: number;
  message: string;
  tone: "info" | "error";
}

const VISIBLE_MS = 4500;

class Toasts {
  items = $state<Toast[]>([]);
  #nextId = 0;

  show(message: string, tone: Toast["tone"] = "info"): void {
    const id = ++this.#nextId;
    this.items = [...this.items, { id, message, tone }];
    setTimeout(() => this.dismiss(id), VISIBLE_MS);
  }

  error(message: string): void {
    this.show(message, "error");
  }

  dismiss(id: number): void {
    this.items = this.items.filter((t) => t.id !== id);
  }
}

export const toasts = new Toasts();
