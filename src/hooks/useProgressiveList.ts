import { useState, useEffect, useRef } from "react";

export interface UseProgressiveListOptions {
  /** How many items to display immediately on initial mount or sort/filter change. Default is 12. */
  initialBatchSize?: number;
  /** How many additional items to stream per tick. Default is 6. */
  streamBatchSize?: number;
  /** Delay in milliseconds between progressive stream ticks. Default is 30ms. */
  tickMs?: number;
  /** Optional dependency or key to force reset (e.g. active sort or category filter). */
  resetKey?: string | number;
}

export interface UseProgressiveListReturn<T> {
  visibleItems: T[];
  visibleCount: number;
  totalCount: number;
  isStreaming: boolean;
  loadMore: () => void;
  showAll: () => void;
}

export function useProgressiveList<T>(
  items: T[],
  options: UseProgressiveListOptions = {}
): UseProgressiveListReturn<T> {
  const {
    initialBatchSize = 12,
    streamBatchSize = 6,
    tickMs = 30,
    resetKey,
  } = options;

  const [visibleCount, setVisibleCount] = useState<number>(() =>
    Math.min(initialBatchSize, items.length)
  );

  const prevItemsRef = useRef(items);
  const prevResetKeyRef = useRef(resetKey);

  // When items or resetKey changes (e.g. user toggled filter or sort),
  // instantly show the top items that fit the window for that sort/filter.
  useEffect(() => {
    const isNewList =
      prevItemsRef.current !== items || prevResetKeyRef.current !== resetKey;

    if (isNewList) {
      prevItemsRef.current = items;
      prevResetKeyRef.current = resetKey;
      setVisibleCount(Math.min(initialBatchSize, items.length));
    }
  }, [items, resetKey, initialBatchSize]);

  // Progressive streaming loop: renders the remaining items smoothly
  // one batch at a time to prevent UI freezing and ensure 60fps interaction.
  useEffect(() => {
    if (visibleCount >= items.length) {
      return;
    }

    const timer = setTimeout(() => {
      setVisibleCount((current) =>
        Math.min(current + streamBatchSize, items.length)
      );
    }, tickMs);

    return () => clearTimeout(timer);
  }, [visibleCount, items.length, streamBatchSize, tickMs]);

  const loadMore = () => {
    setVisibleCount((current) =>
      Math.min(current + streamBatchSize * 2, items.length)
    );
  };

  const showAll = () => {
    setVisibleCount(items.length);
  };

  const visibleItems = items.slice(0, visibleCount);

  return {
    visibleItems,
    visibleCount,
    totalCount: items.length,
    isStreaming: visibleCount < items.length,
    loadMore,
    showAll,
  };
}
