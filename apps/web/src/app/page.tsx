'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';

export default function HomePage() {
  const router = useRouter();

  useEffect(() => {
    router.replace('/contracts');
  }, [router]);

  return <div data-testid="root-redirect" />;
}
