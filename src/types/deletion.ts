export interface DeletionFailure {
  path: string;
  reason: string;
}

export interface DeletionResult {
  deleted: string[];
  failures: DeletionFailure[];
}
