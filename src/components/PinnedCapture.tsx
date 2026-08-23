import { useEffect, useState } from "react";
import { closePinnedCapture, getPinnedCapture, nativeErrorMessage, type PinnedCapture as PinnedCaptureData } from "../lib/nativeCapture";

type Props = {
  id: string;
};

type PinState =
  | { kind: "loading" }
  | { kind: "ready"; capture: PinnedCaptureData }
  | { kind: "error"; message: string };

export default function PinnedCapture({ id }: Props) {
  const [state, setState] = useState<PinState>({ kind: "loading" });

  useEffect(() => {
    let active = true;
    void getPinnedCapture(id)
      .then((capture) => {
        if (!active) return;
        setState(capture ? { kind: "ready", capture } : { kind: "error", message: "This pinned capture is no longer available." });
      })
      .catch((error: unknown) => {
        if (active) setState({ kind: "error", message: nativeErrorMessage(error, "Could not load the pinned capture.") });
      });
    return () => {
      active = false;
    };
  }, [id]);

  const close = () => {
    void closePinnedCapture(id).catch((error: unknown) => {
      setState({ kind: "error", message: nativeErrorMessage(error, "Could not close the pinned capture.") });
    });
  };

  if (state.kind === "loading") return <div className="pin-state">Loading pinned capture…</div>;
  if (state.kind === "error") {
    return (
      <div className="pin-state pin-error">
        <strong>Pin unavailable</strong>
        <span>{state.message}</span>
        <button className="btn" onClick={close}>Close</button>
      </div>
    );
  }

  return (
    <div className="pin-view">
      <img
        src={`data:image/png;base64,${state.capture.pngBase64}`}
        width={state.capture.width}
        height={state.capture.height}
        alt={state.capture.title}
      />
      <button className="pin-close" onClick={close} aria-label="Close pinned capture" title="Close pinned capture">
        ×
      </button>
    </div>
  );
}
