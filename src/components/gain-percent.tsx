import * as React from 'react';
import { Icons } from '@/components/icons';
import { cn } from '@/lib/utils';
import NumberFlow from '@number-flow/react';

interface GainPercentProps extends React.HTMLAttributes<HTMLDivElement> {
  value: number;
  animated?: boolean;
}

export function GainPercent({ value, animated = false, className, ...props }: GainPercentProps) {
  return (
    <div
      className={cn(
        'amount flex flex-col items-end text-right',
        className,
        value === 0 ? 'text-foreground' : value > 0 ? 'text-success' : 'text-destructive',
      )}
      {...props}
    >
      <div className="flex items-center">
        {value > 0 ? (
          <Icons.ArrowUp className="h-3 w-3" />
        ) : value < 0 ? (
          <Icons.ArrowDown className="h-3 w-3" />
        ) : (
          <Icons.ArrowRight className="h-3 w-3" />
        )}
        <NumberFlow
          value={value / 100}
          animated={animated}
          format={{
            style: 'percent',
            minimumFractionDigits: 2,
            maximumFractionDigits: 2,
          }}
        />
      </div>
    </div>
  );
}
