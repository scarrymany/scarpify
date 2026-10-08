/** Transient interface state shared by distant components. */

interface ConfirmRequest {
  title: string;
  body: string;
  confirmLabel: string;
  resolve: (confirmed: boolean) => void;
}

class Ui {
  /** Playlist whose title is being edited in place. */
  renaming = $state<string | null>(null);
  confirmRequest = $state<ConfirmRequest | null>(null);

  confirm(title: string, body: string, confirmLabel: string): Promise<boolean> {
    this.confirmRequest?.resolve(false);
    return new Promise((resolve) => {
      this.confirmRequest = { title, body, confirmLabel, resolve };
    });
  }

  answer(confirmed: boolean): void {
    this.confirmRequest?.resolve(confirmed);
    this.confirmRequest = null;
  }
}

export const ui = new Ui();
