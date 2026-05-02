import type { UserType } from './schemas';

interface Repository {
    findById(id: string): Promise<UserType | null>;
    save(user: UserType): Promise<void>;
}

export class UserRepository implements Repository {
    async findById(id: string): Promise<UserType | null> {
        return fetchFromDb(id);
    }

    async save(user: UserType): Promise<void> {
        await persistToDb(user);
    }
}

export class UserService extends UserRepository {
    async fetchUser(): Promise<UserType | null> {
        return this.findById('current');
    }
}

export function fetchUser(): Promise<UserType | null> {
    const service = new UserService();
    return service.fetchUser();
}

function fetchFromDb(id: string): Promise<UserType | null> {
    return Promise.resolve(null);
}

function persistToDb(user: UserType): Promise<void> {
    return Promise.resolve();
}
