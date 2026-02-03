public class envleak {
    public static void main(String[] args) {
        String flag = System.getenv("FLAG");
        if (flag != null) System.out.println(flag);
    }
}
