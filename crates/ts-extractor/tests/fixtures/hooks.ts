import { useState, useEffect } from 'react';
import { fetchUser } from './services';
import type { UserType } from './schemas';

export function useCurrentUser(): UserType | null {
    const [user, setUser] = useState<UserType | null>(null);
    useEffect(() => {
        fetchUser().then(setUser);
    }, []);
    return user;
}

export function useToggle(initial: boolean): [boolean, () => void] {
    const [value, setValue] = useState(initial);
    const toggle = () => setValue(!value);
    return [value, toggle];
}
