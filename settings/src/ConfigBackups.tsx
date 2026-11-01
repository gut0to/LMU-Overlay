import { backupTime, type ConfigBackup } from "./backups";

export function ConfigBackups(props: {
  items: ConfigBackup[];
  loading: boolean;
  busy: boolean;
  error: string | null;
  onCreate: () => void;
  onRefresh: () => void;
  onReview: (id: string) => void;
}) {
  return (
    <div className="configBackups">
      <p className="previewHint">
        Keep up to five copies of the saved configuration. Pending editor
        changes are not included until you save them. Imports and resets also
        create a backup.
      </p>
      <div className="buttonRow">
        <button
          className="primaryButton"
          disabled={props.busy}
          onClick={props.onCreate}
        >
          Back up saved settings
        </button>
        <button
          className="secondaryButton"
          disabled={props.busy || props.loading}
          onClick={props.onRefresh}
        >
          {props.loading ? "Loading backups…" : "Refresh backups"}
        </button>
      </div>
      {props.error && (
        <p className="inlineError" role="alert">
          {props.error}
        </p>
      )}
      {!props.loading && props.items.length === 0 && (
        <p className="previewHint">
          No configuration backups yet. Create one before experimenting with a
          setup.
        </p>
      )}
      <ul className="backupList" aria-label="Saved configuration backups">
        {props.items.map((backup) => (
          <li key={backup.id}>
            <span>
              <strong>{backupTime(backup.modified_ms)}</strong>
              <small>{(backup.size_bytes / 1024).toFixed(1)} KiB</small>
            </span>
            <button
              className="secondaryButton"
              disabled={props.busy}
              onClick={() => props.onReview(backup.id)}
              aria-label={`Review backup from ${backupTime(backup.modified_ms)}`}
            >
              Review backup
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
