global with sharing class SomeCallable implements Callable {
    public Object call(String param1, Map<String, Object> param2) {
        throw new SomeCallableException('Really, no, please, no no no you did not.');
    }

    public class SomeCallableException extends Exception {}
}
