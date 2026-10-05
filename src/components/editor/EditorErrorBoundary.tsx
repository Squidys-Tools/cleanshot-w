import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  /** Changing this resets the boundary, so switching captures recovers. */
  resetKey?: string | number;
}

interface State {
  error: Error | null;
}

/**
 * Contains a crash inside the tldraw editor subtree.
 *
 * The editor is the product, and until this existed a failure inside it had no
 * boundary to land in: React tore down the whole tree, or the canvas simply
 * stayed blank with nothing in the UI or the logs to explain why. Both outcomes
 * cost the release gate an entire diagnostic pass.
 *
 * The fallback states the error in the interface on purpose. A tester can now
 * read what went wrong, and `scripts/diagnose-packaged.mjs` can scrape it from
 * a packaged build where devtools are unavailable.
 */
export default class EditorErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("The editor failed to render.", error, info.componentStack);
  }

  componentDidUpdate(prev: Props): void {
    if (this.state.error && prev.resetKey !== this.props.resetKey) {
      this.setState({ error: null });
    }
  }

  private readonly retry = () => this.setState({ error: null });

  render(): ReactNode {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="editor-crash" role="alert">
        <div className="editor-crash-card">
          <h2 className="editor-crash-title">The editor could not be displayed</h2>
          <p className="editor-crash-body">
            Something went wrong while drawing the canvas. The rest of the app is still usable, and your
            captures are safe on disk.
          </p>
          <p className="editor-crash-detail" data-testid="editor-crash-detail">
            {error.message || String(error)}
          </p>
          <div className="editor-crash-actions">
            <button type="button" className="btn primary" onClick={this.retry}>
              Try again
            </button>
          </div>
        </div>
      </div>
    );
  }
}
