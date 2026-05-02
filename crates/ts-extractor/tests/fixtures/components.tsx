import React from 'react';
import { useCurrentUser } from './hooks';
import type { UserType } from './schemas';

interface ProfileProps {
    userId: string;
    showEmail: boolean;
}

export function ProfileCard({ userId, showEmail }: ProfileProps): React.ReactElement {
    const user = useCurrentUser();
    return (
        <div className="profile">
            <h1>{user?.name}</h1>
            {showEmail && <span>{user?.email}</span>}
        </div>
    );
}

export default ProfileCard;
