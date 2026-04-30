import { useState, useRef, type ReactNode, type MouseEvent, type ReactElement, cloneElement, isValidElement } from "react";

interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
  className?: string;
}

export default function Tooltip({ content, children, className = "" }: TooltipProps) {
  const [isVisible, setIsVisible] = useState(false);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const showTooltip = (e: MouseEvent<HTMLElement>) => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    const rect = e.currentTarget.getBoundingClientRect();
    setPosition({
      x: rect.left + rect.width / 2,
      y: rect.bottom + 8,
    });
    setIsVisible(true);
  };

  const hideTooltip = () => {
    timeoutRef.current = setTimeout(() => {
      setIsVisible(false);
    }, 100);
  };

  if (!isValidElement(children)) {
    return <>{children}</>;
  }

  const child = children as ReactElement<{
    onMouseEnter?: (e: MouseEvent<HTMLElement>) => void;
    onMouseLeave?: (e: MouseEvent<HTMLElement>) => void;
    [key: string]: unknown;
  }>;

  const enhancedChild = cloneElement(child, {
    onMouseEnter: (e: MouseEvent<HTMLElement>) => {
      child.props.onMouseEnter?.(e);
      showTooltip(e);
    },
    onMouseLeave: (e: MouseEvent<HTMLElement>) => {
      child.props.onMouseLeave?.(e);
      hideTooltip();
    },
  });

  return (
    <span className={`relative inline-block ${className}`}>
      {enhancedChild}
      {isVisible && (
        <span
          className="fixed z-50 pointer-events-none"
          style={{
            left: position.x,
            top: position.y,
            transform: "translateX(-50%)",
          }}
          onMouseEnter={() => {
            if (timeoutRef.current) clearTimeout(timeoutRef.current);
          }}
          onMouseLeave={hideTooltip}
        >
          <span className="bg-gray-900 dark:bg-navy-800 text-white text-xs rounded-lg shadow-xl p-3 max-w-xs border border-gray-700 dark:border-navy-600">
            {content}
          </span>
          <span className="absolute left-1/2 -translate-x-1/2 -top-1 w-0 h-0 border-l-4 border-r-4 border-b-4 border-l-transparent border-r-transparent border-b-gray-900 dark:border-b-navy-800" />
        </span>
      )}
    </span>
  );
}
